# egalax-rs

An input driver for our iiyama ProLite T1930S monitor with integrated touchscreen. 
Translates from the touchscreen's USB protocol into commands to control a virtual mouse with uinput.

## Build & Install

### Dependencies
- xrandr: get screen size information.
- X11: dependency of xrandr
- evdev: interact with uinput.

#### openSUSE
```bash
$ sudo zypper install libXrandr-devel libX11-devel libevdev-devel
```

#### Ubuntu
```bash
$ sudo apt install libudev-dev libxrandr-dev libx11-dev libevdev-dev 
```

Then to build and install the program.
```
$ cargo build
$ cargo install --path .
```

Put the udev `.rules` files in `/etc/udev/rules.d` and the `egalax.service` unit file in `/etc/systemd/system/`.

## File Structure

- `c_src/` - C files to test some libc/kernel APIs.
- `dis/` - Ghidra project to disassemble the manufacturer's eGTouchD driver.
- `logs/` - Various log outputs which are discussed below.
- `Guide/` - Resources from the manufacturer. PDFs which describe the the monitor and a raw binary protocol of the touchscreen.  
           Though, our monitor actually uses a different protocol as discussed below.
- `linux_config/` - Config files to automatically start the driver when the USB cable is plugged in.
- `media/` - Resources for the calibration tool.

## Background
We had an old iiyama ProLite T1930S monitor with an integrated touchscreen lying around which we didn't know how to use. There was probably a driver of the manufacturer that we could install, but wanting to learn more about Linux we decided to write our own userspace driver for it.

A *userspace driver* is a driver that runs as a normal user program, interacting with kernel APIs to implement the driver behavior. We decided on this approach as there is less danger of breaking things and we can use any language that can do system calls. 
Devices like mice, keyboards and touchscreens are collectively referred to as *human interface devices* (HID) and they are handled by the Linux [input subsystem](https://docs.kernel.org/input/input.html) which contains a generic usbhid driver that applies to many input devices. For userspace drivers the system exposes [uinput](https://docs.kernel.org/input/uinput.html) which allows creating and controlling virtual input devices.

HID devices communicate via *HID reports*, binary messages that describe the state of the device. Their schema is described by *HID report descriptors* and in general the usbhid driver's job is to parse those descriptors and then interpret HID reports into input events.
Since we want to write our own driver we use the kernel's [hidraw interface](https://docs.kernel.org/hid/hidraw.html) to get access to the original HID reports.

<table>
<tr>
    <td><b>Generic USB Mouse</b></td>
    <td><b>Plan for egalax-rs</b></td>
</tr>
<tr>
    <td>

- [usbhid](https://docs.kernel.org/input/input.html#hid-generic) communicates with device over USB and generates input events.
- [evdev](https://docs.kernel.org/input/input.html#evdev) is the interface for userspace applications to receive input events. All the event nodes in /dev/input/ belong to it.
- The xorg drivers `xf86-input-{evdev,libinput}` are wrappers around evdev to relay input events to the X server.
- finally, the event reaches the X server and then the client application that will react to it.
    </td>
<td>

- Instead of relying on usbhid we get the raw HID report data via the [hidraw driver](https://docs.kernel.org/hid/hidraw.html).
- We interpret the HID report and generate input events that we inject back into the input subsystem using uinput.
- Then evdev will present these events to userspace drivers as before.
</td>
</tr>
<tr>
    <td>

```
X server & client
    ^
    |
xf86-input-{evdev,libinput}
    ^
    | via /dev/input/eventX
    |
evdev
    ^
    |
kernel (via usbhid)
    ^
    |
physical device 
```

</td>
<td>

```
X server & client
    ^
    |
xf86-input-{evdev,libinput}
              ^
              | via /dev/eventX
              \ 
egalax-rs ---> evdev (uinput)
    ^               
    | via /dev/hidrawX 
    |
kernel (via hidraw driver)
    ^
    |
physical device
```
</td>
</tr>
</table>

### Getting the hidraw device
The [hidraw](https://docs.kernel.org/hid/hidraw.html) documentation mentions the following.
```
Hidraw uses a dynamic major number, meaning that udev should be relied on to create hidraw device nodes.
```

This affected me as I used to use the first hidraw device `/dev/hidraw0` to read the touchscreen input but currently it is taken up by the buttons on my external USB speakers.
For development it's easier if we have a static device node. For that we use the folloing udev rules in the file `linux_config/51-hidraw.rules`
```
SUBSYSTEM=="hidraw", ACTION=="add", SUBSYSTEMS=="usb", ATTRS{idProduct}=="0001", ATTRS{idVendor}=="0eef", GROUP="input", SYMLINK+="hidraw.egalax"
```
When the touchscreen is plugged in this creates the device node `/dev/hidraw.egalax` from which we can read the raw HID reports.

We can get the product and vendor ID by querying the connected USB devices using `lsusb`. This also shows us the USB bus and device ID that we need in the following.
```
$ lsusb
[...]
Bus 005 Device 027: ID 0eef:0001 D-WAV Scientific Co., Ltd Titan6001 Surface Acoustic Wave Touchscreen Controller [eGalax]
```

### Binary Protocol of the Touchscreen

There are several ways we can get an idea about the binary protocol that the touchscreen uses.

1. Educated guesses when looking at the HID reports.
2. Reading the HID report descriptor.
3. Disassembling the manufacturer's driver.

#### 1. Interpreting HID reports
We can print the HID reports from our hidraw device node using `xxd`. Since we are looking for patterns we print it in binary.
First touching and releasing the upper-left corner, and then touching and releasing the lower-right corner of the monitor results in the following output.

```
$ xxd -b /dev/hidraw.egalax
00000000: 00000010 00000011 01100010 00000001 11100011 00000001  ..b...
00000006: 00000010 00000011 01100011 00000001 11100010 00000001  ..c...
0000000c: 00000010 00000011 01100001 00000001 11100010 00000001  ..a...
00000012: 00000010 00000011 01100001 00000001 11100000 00000001  ..a...
00000018: 00000010 00000011 01100001 00000001 11011111 00000001  ..a...
0000001e: 00000010 00000011 01100000 00000001 11100000 00000001  ..`...
00000024: 00000010 00000010 01100000 00000001 11100000 00000001  ..`...
0000002a: 00000010 00000011 01111000 00001101 11101110 00001101  ..x...
00000030: 00000010 00000011 01110100 00001101 11101100 00001101  ..t...
00000036: 00000010 00000011 01110010 00001101 11101011 00001101  ..r...
0000003c: 00000010 00000011 01110010 00001101 11101011 00001101  ..r...
00000042: 00000010 00000011 01110011 00001101 11101011 00001101  ..s...
00000048: 00000010 00000011 01110000 00001101 11101010 00001101  ..p...
0000004e: 00000010 00000011 01101111 00001101 11101010 00001101  ..o...
00000054: 00000010 00000011 01101111 00001101 11101011 00001101  ..o...
0000005a: 00000010 00000011 01101110 00001101 11101110 00001101  ..n...
00000060: 00000010 00000011 01101101 00001101 11101110 00001101  ..m...
00000066: 00000010 00000010 01101101 00001101 11101110 00001101  ..m...
```

Each HID report consists of 6 bytes, which we number `m[0]` to `m[5]`.
We can see the following pattern, where the first two bytes except the touch flag are probably some constant metadata or simply padding.

```
m[0]        = 0x2 is constant
m[1][7:1]   = 0b0000001_ is constant
m[1][0]     = indicates whether we are touching or releasing a finger

m[3] | m[2] = the x position of the touch
m[5] | m[4] = the y position of the touch
```

#### 2. Reading the HID report descriptor

Instead of crudely reading the actual HID reports we can also take a look at the HID report descriptors that describe their schema. 
This can be done using the `usbhid-dump` utitilty. We just need to point it to the correct device using the USB bus and device ID from above.

```
$ sudo usbhid-dump -a 5:27 -p
005:032:000:DESCRIPTOR         1727744627.383960
 05 01 09 01 A1 01 85 01 09 01 A1 00 05 09 19 01
 29 02 15 00 25 01 95 02 75 01 81 02 95 01 75 06
 81 01 05 01 09 30 09 31 16 2A 00 26 BD 07 36 00
 00 46 FF 0F 66 00 00 75 10 95 02 81 02 C0 C0 05
 0D 09 04 A1 01 85 02 09 20 A1 00 09 42 09 32 15
 00 25 01 95 02 75 01 81 02 95 06 75 01 81 03 05
 01 09 30 75 10 95 01 A4 55 00 65 00 36 00 00 46
 00 00 16 1E 00 26 C8 0F 81 02 09 31 16 3C 00 26
 C3 0F 36 00 00 46 00 00 81 02 B4 C0 C0
```

This series of hex bytes doesn't really tell us anything though.
But as explained in the [HID introduction of the Linux kernel documentation](https://docs.kernel.org/hid/hidintro.html) we can paste them (exclude the header) into a [USB descriptor parser](http://eleccelerator.com/usbdescreqparser/), which gives us the following long list describing the HID reports.

```
0x05, 0x01,        // Usage Page (Generic Desktop Ctrls)
0x09, 0x01,        // Usage (Pointer)
0xA1, 0x01,        // Collection (Application)
0x85, 0x01,        //   Report ID (1)
0x09, 0x01,        //   Usage (Pointer)
0xA1, 0x00,        //   Collection (Physical)
0x05, 0x09,        //     Usage Page (Button)
0x19, 0x01,        //     Usage Minimum (0x01)
0x29, 0x02,        //     Usage Maximum (0x02)
0x15, 0x00,        //     Logical Minimum (0)
0x25, 0x01,        //     Logical Maximum (1)
0x95, 0x02,        //     Report Count (2)
0x75, 0x01,        //     Report Size (1)
0x81, 0x02,        //     Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
0x95, 0x01,        //     Report Count (1)
0x75, 0x06,        //     Report Size (6)
0x81, 0x01,        //     Input (Const,Array,Abs,No Wrap,Linear,Preferred State,No Null Position)
0x05, 0x01,        //     Usage Page (Generic Desktop Ctrls)
0x09, 0x30,        //     Usage (X)
0x09, 0x31,        //     Usage (Y)
0x16, 0x2A, 0x00,  //     Logical Minimum (42)
0x26, 0xBD, 0x07,  //     Logical Maximum (1981)
0x36, 0x00, 0x00,  //     Physical Minimum (0)
0x46, 0xFF, 0x0F,  //     Physical Maximum (4095)
0x66, 0x00, 0x00,  //     Unit (None)
0x75, 0x10,        //     Report Size (16)
0x95, 0x02,        //     Report Count (2)
0x81, 0x02,        //     Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
0xC0,              //   End Collection
0xC0,              // End Collection
0x05, 0x0D,        // Usage Page (Digitizer)
0x09, 0x04,        // Usage (Touch Screen)
0xA1, 0x01,        // Collection (Application)
0x85, 0x02,        //   Report ID (2)
0x09, 0x20,        //   Usage (Stylus)
0xA1, 0x00,        //   Collection (Physical)
0x09, 0x42,        //     Usage (Tip Switch)
0x09, 0x32,        //     Usage (In Range)
0x15, 0x00,        //     Logical Minimum (0)
0x25, 0x01,        //     Logical Maximum (1)
0x95, 0x02,        //     Report Count (2)
0x75, 0x01,        //     Report Size (1)
0x81, 0x02,        //     Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
0x95, 0x06,        //     Report Count (6)
0x75, 0x01,        //     Report Size (1)
0x81, 0x03,        //     Input (Const,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
0x05, 0x01,        //     Usage Page (Generic Desktop Ctrls)
0x09, 0x30,        //     Usage (X)
0x75, 0x10,        //     Report Size (16)
0x95, 0x01,        //     Report Count (1)
0xA4,              //     Push
0x55, 0x00,        //       Unit Exponent (0)
0x65, 0x00,        //       Unit (None)
0x36, 0x00, 0x00,  //       Physical Minimum (0)
0x46, 0x00, 0x00,  //       Physical Maximum (0)
0x16, 0x1E, 0x00,  //       Logical Minimum (30)
0x26, 0xC8, 0x0F,  //       Logical Maximum (4040)
0x81, 0x02,        //       Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
0x09, 0x31,        //       Usage (Y)
0x16, 0x3C, 0x00,  //       Logical Minimum (60)
0x26, 0xC3, 0x0F,  //       Logical Maximum (4035)
0x36, 0x00, 0x00,  //       Physical Minimum (0)
0x46, 0x00, 0x00,  //       Physical Maximum (0)
0x81, 0x02,        //       Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
0xB4,              //     Pop
0xC0,              //   End Collection
0xC0,              // End Collection
```

I'm still not sure what everything here means but apparently there are two kinds of reports, one being a "pointer", the other a "touch screen". 

Both have buttons with binary state (logical minimum = 0 & logical maximum = 1) that are represented by 2 bits (report count = 2 & report size = 1). Then follow 6 constant bits and then X and Y coordinates with 16 bits each (I assume that's what the push & pop instructions mean for the second report).

In total this maps nicely to the values we see from the hidraw interface. The hidraw documentation tells us that "On a device which uses numbered reports, the first byte of the returned data will be the report number; the report data follows, beginning in the second byte."

That means the first byte `m[0]` that was a constant `0x2` in the HID reports above, designates the second type of HID reports for the touch screen interface, the second byte `m[1]` includes the two bits for the buttons plus 6 constant bits and the next 4 bytes are the X and Y value, respectively.
I'm not sure what the second bit for the buttons means. It might be the "In Range" usage mentioned in the descriptor so maybe it's only 0 if I somehow produce an input that is out of range, which I have not been able to yet.

Unfortunately, the screen reports wrong numbers (e.g. X values are supposed to range from 30 to 4040) while in reality they seem to range from 300 to 3700.
So manual calibration is unavoidable if we want to avoid magic numbers in our binary.


#### 2. Using the manufacturer's driver

While reading the raw HID reports and the HID report descriptors gives us a rough picture of how to parse the data coming in over the hidraw interface, we can get an even better idea straight from the manufacturer by checking what [their driver](https://www.eeti.com/drivers_Linux.html) does.
We use Ghidra to reverse engineer some functions in their `eGTouchU` driver. It helps that they have extensive debug logs for each function entry and exit.

TODO describe the analysis 

To summarize:
1. The first HID report byte `0x2` corresponds to the normal kind of message expected 
from the touch screen.
2. The first bit of the second byte `m[1][0]` is the status bit for a touch event happening.
3. The next two bits of the second byte `m[1][1:3]` encode the *resolution* of the touch screen. For our monitor it is a constant `0b01`, which according to the "Software Programming Guide" corresponds to 12 bits of resolution in the X and Y axes.
I'm not sure why the HID report descriptor instead defines an "In Range" bit and why there is only one bit instead of two. I presume the manufacturer just adapted the descriptor for each monitor to fit the Linux HID requirements.
4. Bytes 3 to 6 are then the X and Y position (with a resolution of 12 bits) in little-endian byte order.

### Log Dumps
I recorded some output from connecting the touchscreen & touch interactions.
In these examples the touchscreen was always assigned the node `/dev/hidraw.egalax` for raw events, and `/dev/input/event19` by evdev.

1. #### `hidraw.bin`
Contains the HID reports that the touchscreen sends over USB when a touch interaction happens.
Result of touching the 4 corners of the screen.

 ```tee hidraw.bin < /dev/hidraw.egalax | hexdump -C```


2. #### `xxd.log`.
The binary data above visualized with xxd. 
Already possible to deduce binary format from this. 

```xxd -b hidraw.bin > xxd.log```


3. #### `event19.bin`
The evdev events generated by `usbhid`.
It turns out the usbhid driver can already handle the touchscreen and generates evdev events for it, but in the default configuration the X server was not picking them up.

 ```tee event19.bin < /dev/input/event19 | hexdump -C```

4. #### `recording.txt`
The evdev events as reported by `evemu-record`.

 ```evemu-record /dev/input/event19```

AFAIU, the kernel already has a driver for touchscreens (usbhid) and it does generate an input device and exposes it in `/dev/event19`.
This shows the evdev events emitted by the input device, which are similar to those we create with our driver.
 
 - excerpts from `/var/log/Xorg.0.log` in `xorg-libinput.log` and `xorg-evdev.log`

 At first, I did not have the xorg evdev driver `xf86-input-evdev` installed. So after putting the `53-egalax-usbhid.conf` in `/usr/share/X11/xorg.conf.d` (and rebooting/restarting X) you can see in the log that it falls back on libinput, which in turn does not seem to like the touchscreen. The screen might just be too old for libinput.

 After installing the evdev driver it works, the touchscreen is registered in xinput and moves the cursor. However, it moves the cursor over the whole virtual screen space (all connected outputs combined) and is horribly calibrated, so we don't use it.

## Ways to get information
- `lsusb` lists all usb devices. Use `-v` (preferably while filtering out one device with `-s`) to get the whole descriptor tree for usb devices.

- /sys

As described [here](http://cholla.mmto.org/computers/usb/OLD/tutorial_usbloger.html) "Sysfs is a virtual filesystem exported by the kernel, similar to /proc. The files in Sysfs contain information about devices and drivers. Some files in Sysfs are even writable, for configuration and control of devices attached to the system. Sysfs is always mounted on /sys."
- /proc

similar to lsusb, lists in more detail which devices are connected. Also lists to which `/dev/input/eventX` node it emits event (but this info can also be found in /sys)
- using the `usbutils` package
```bash
$ usb-devices
```
Probably scrapes /sys (similar to explained [here](https://unix.stackexchange.com/a/60080)). Quickly shows which driver is used.

- `evemu-record` can pretty-print evdev messages generated by an eventX node
- `/var/log/Xorg.0.log` for xorg logs, e.g. when it recognizes a new input device and tries to find a driver for it
- `/usr/share/X11/xorg.conf.d/` can place *.conf file in there to tell xorg which devices should use which drivers. [.conf file format](https://www.x.org/releases/current/doc/man/man5/xorg.conf.5.xhtml). There are several such directories but this one supports reading the config files again on every device connection so you don't have to reboot all the time.
- `dmesg` kernel messages. The first one to tell you when a new device is connected.
- `/etc/modprobe.d/blacklist` blacklist for kernel modules
- 

## Using above commands to get info about egalax touchscreen (while `usbtouchscreen` blacklisted)
1. plug in egalax usb cable 
2. `dmesg`
```
[ 1449.625510] usb 3-2: new full-speed USB device number 11 using xhci_hcd
[ 1449.782308] usb 3-2: New USB device found, idVendor=0eef, idProduct=0001, bcdDevice= 1.00
[ 1449.782317] usb 3-2: New USB device strings: Mfr=1, Product=2, SerialNumber=0
[ 1449.782321] usb 3-2: Product: USB TouchController
[ 1449.782323] usb 3-2: Manufacturer: eGalax Inc.
[ 1449.796734] input: eGalax Inc. USB TouchController as /devices/pci0000:00/0000:00:08.1/0000:07:00.3/usb3/3-2/3-2:1.0/0003:0EEF:0001.0002/input/input20
[ 1449.797028] input: eGalax Inc. USB TouchController Touchscreen as /devices/pci0000:00/0000:00:08.1/0000:07:00.3/usb3/3-2/3-2:1.0/0003:0EEF:0001.0002/input/input21
[ 1449.797187] hid-generic 0003:0EEF:0001.0002: input,hidraw0: USB HID v2.10 Pointer [eGalax Inc. USB TouchController] on usb-0000:07:00.3-2/input0
```
2. `cat /proc/bus/input/devices` shows the `/dev/input/eventX` nodes associated with each device.
```
I: Bus=0003 Vendor=0eef Product=0001 Version=0210
N: Name="eGalax Inc. USB TouchController"
P: Phys=usb-0000:07:00.3-2/input0
S: Sysfs=/devices/pci0000:00/0000:00:08.1/0000:07:00.3/usb3/3-2/3-2:1.0/0003:0EEF:0001.0002/input/input20
U: Uniq=
H: Handlers=event18 mouse2 js0
B: PROP=0
B: EV=1b
B: KEY=30000 0 0 0 0
B: ABS=3
B: MSC=10

I: Bus=0003 Vendor=0eef Product=0001 Version=0210
N: Name="eGalax Inc. USB TouchController Touchscreen"
P: Phys=usb-0000:07:00.3-2/input0
S: Sysfs=/devices/pci0000:00/0000:00:08.1/0000:07:00.3/usb3/3-2/3-2:1.0/0003:0EEF:0001.0002/input/input21
U: Uniq=
H: Handlers=event19 mouse3
B: PROP=0
B: EV=1b
B: KEY=401 0 0 0 0 0
B: ABS=3
B: MSC=10
```
2. `lsusb` shows the bus and device number
```
Bus 003 Device 011: ID 0eef:0001 D-WAV Scientific Co., Ltd Titan6001 Surface Acoustic Wave Touchscreen Controller [eGalax]
```
2. `/dev/input/event18`, `/dev/input/event19`, `/dev/input/mouse2`, and `/dev/hidraw0` are created. Using `cat` on them and then touching the screen shows events in all, except `event18` which is the event node of `USB TouchController`, the rest appear to belong to `USB TouchController Touchscreen`.
3. `evemu-record /dev/input/event19` gives me the expected output for touchscreens with absolute axis values.
```
# EVEMU 1.3
# Kernel: 5.16.12-arch1-1
# DMI: dmi:bvnLENOVO:bvrR1MET43W(1.13):bd11/05/2021:br1.13:efr1.13:svnLENOVO:pn21A1S00D00:pvrThinkPadP14sGen2a:rvnLENOVO:rn21A1S00D00:rvrNotDefined:cvnLENOVO:ct10:cvrNone:skuLENOVO_MT_21A1_BU_Think_FM_ThinkPadP14sGen2a:
# Input device name: "eGalax Inc. USB TouchController Touchscreen"
# Input device ID: bus 0x03 vendor 0xeef product 0x01 version 0x210
# Supported events:
#   Event type 0 (EV_SYN)
#     Event code 0 (SYN_REPORT)
#     Event code 1 (SYN_CONFIG)
#     Event code 2 (SYN_MT_REPORT)
#     Event code 3 (SYN_DROPPED)
#     Event code 4 ((null))
#     Event code 5 ((null))
#     Event code 6 ((null))
#     Event code 7 ((null))
#     Event code 8 ((null))
#     Event code 9 ((null))
#     Event code 10 ((null))
#     Event code 11 ((null))
#     Event code 12 ((null))
#     Event code 13 ((null))
#     Event code 14 ((null))
#     Event code 15 (SYN_MAX)
#   Event type 1 (EV_KEY)
#     Event code 320 (BTN_TOOL_PEN)
#     Event code 330 (BTN_TOUCH)
#   Event type 3 (EV_ABS)
#     Event code 0 (ABS_X)
#       Value     2235
#       Min         30
#       Max       4040
#       Fuzz         0
#       Flat         0
#       Resolution   0
#     Event code 1 (ABS_Y)
#       Value     1755
#       Min         60
#       Max       4035
#       Fuzz         0
#       Flat         0
#       Resolution   0
#   Event type 4 (EV_MSC)
#     Event code 4 (MSC_SCAN)
# Properties:
N: eGalax Inc. USB TouchController Touchscreen
I: 0003 0eef 0001 0210
P: 00 00 00 00 00 00 00 00
B: 00 0b 00 00 00 00 00 00 00
B: 01 00 00 00 00 00 00 00 00
B: 01 00 00 00 00 00 00 00 00
B: 01 00 00 00 00 00 00 00 00
B: 01 00 00 00 00 00 00 00 00
B: 01 00 00 00 00 00 00 00 00
B: 01 01 04 00 00 00 00 00 00
B: 01 00 00 00 00 00 00 00 00
B: 01 00 00 00 00 00 00 00 00
B: 01 00 00 00 00 00 00 00 00
B: 01 00 00 00 00 00 00 00 00
B: 01 00 00 00 00 00 00 00 00
B: 01 00 00 00 00 00 00 00 00
B: 02 00 00 00 00 00 00 00 00
B: 03 03 00 00 00 00 00 00 00
B: 04 10 00 00 00 00 00 00 00
B: 05 00 00 00 00 00 00 00 00
B: 11 00 00 00 00 00 00 00 00
B: 12 00 00 00 00 00 00 00 00
B: 14 00 00 00 00 00 00 00 00
B: 15 00 00 00 00 00 00 00 00
B: 15 00 00 00 00 00 00 00 00
A: 00 30 4040 0 0 0
A: 01 60 4035 0 0 0
################################
#      Waiting for events      #
################################
E: 0.000001 0004 0004 852034	# EV_MSC / MSC_SCAN             852034
E: 0.000001 0001 014a 0001	# EV_KEY / BTN_TOUCH            1
E: 0.000001 0003 0000 2190	# EV_ABS / ABS_X                2190
E: 0.000001 0003 0001 1494	# EV_ABS / ABS_Y                1494
E: 0.000001 0000 0000 0000	# ------------ SYN_REPORT (0) ---------- +0ms
E: 0.005841 0003 0000 2189	# EV_ABS / ABS_X                2189
E: 0.005841 0003 0001 1495	# EV_ABS / ABS_Y                1495
```
4. `evemu-record` on `/dev/input/mouse2` and `/dev/hidraw0` fails, because they are "grabbed"
5. looking in /sys as explained [here](https://unix.stackexchange.com/questions/60078/find-out-which-modules-are-associated-with-a-usb-device) apparently shows that usbtouchscreen is the default driver.

```
$ /sbin/modinfo `cat 3-2\:1.0/modalias`
filename:       /lib/modules/5.16.12-arch1-1/kernel/drivers/input/touchscreen/usbtouchscreen.ko.zst
alias:          mtouchusb
alias:          itmtouch
alias:          touchkitusb
license:        GPL
description:    USB Touchscreen Driver
author:         Daniel Ritz <daniel.ritz@gmx.ch>
srcversion:     58F1DF92EBD267AF56C1783
alias:          usb:v7374p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v04E7p0020d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v1870p0001d*dc*dsc*dp*ic0Aisc00ip00in*
alias:          usb:v10F0p2002d*dc*dsc*dp*ic0Aisc00ip00in*
alias:          usb:v0664p0306d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0664p0309d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v14C8p0003d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v1AC7p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0F92p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v08F2p00F4d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v08F2p00CEd*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v08F2p007Fd*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0DFCp0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v1391p1000d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v6615p0012d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v6615p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v595Ap0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v255Ep0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0AFAp03E8d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0637p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v1234p5678d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v16E3pF9E9d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0403pF9E9d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0596p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v134Cp0004d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v134Cp0003d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v134Cp0002d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v134Cp0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v1234p0002d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v1234p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0EEFp0002d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0EEFp0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0123p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v3823p0002d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v3823p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0EEFp0002d*dc*dsc*dp*ic03isc*ip*in*
alias:          usb:v0EEFp0001d*dc*dsc*dp*ic03isc*ip*in*
depends:
retpoline:      Y
intree:         Y
name:           usbtouchscreen
vermagic:       5.16.12-arch1-1 SMP preempt mod_unload
sig_id:         PKCS#7
signer:         Build time autogenerated kernel key
sig_key:        5B:91:A2:B2:2D:19:EA:F2:23:3D:57:51:3C:19:43:67:96:4B:B5:CB
sig_hashalgo:   sha512
signature:      30:66:02:31:00:DA:1F:9C:3C:B7:EB:10:F6:02:EA:38:64:0A:23:44:
		14:27:FD:CD:73:E2:58:DB:54:4A:61:BA:A3:88:2C:CD:0F:EC:15:87:
		77:5A:31:B4:37:B8:2F:04:A0:54:38:B3:EF:02:31:00:AB:2F:64:87:
		3D:83:2F:15:06:63:4B:D0:C5:C6:41:9C:C8:80:B7:13:B5:CE:F0:31:
		CD:15:8F:39:94:86:38:35:9A:9E:30:67:2E:55:70:79:E4:C9:77:2E:
		56:DB:82:F2
parm:           swap_xy:If set X and Y axes are swapped. (bool)
parm:           hwcalib_xy:If set hw-calibrated X/Y are used if available (bool)
filename:       /lib/modules/5.16.12-arch1-1/kernel/drivers/input/touchscreen/usbtouchscreen.ko.zst
alias:          mtouchusb
alias:          itmtouch
alias:          touchkitusb
license:        GPL
description:    USB Touchscreen Driver
author:         Daniel Ritz <daniel.ritz@gmx.ch>
srcversion:     58F1DF92EBD267AF56C1783
alias:          usb:v7374p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v04E7p0020d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v1870p0001d*dc*dsc*dp*ic0Aisc00ip00in*
alias:          usb:v10F0p2002d*dc*dsc*dp*ic0Aisc00ip00in*
alias:          usb:v0664p0306d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0664p0309d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v14C8p0003d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v1AC7p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0F92p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v08F2p00F4d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v08F2p00CEd*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v08F2p007Fd*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0DFCp0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v1391p1000d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v6615p0012d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v6615p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v595Ap0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v255Ep0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0AFAp03E8d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0637p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v1234p5678d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v16E3pF9E9d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0403pF9E9d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0596p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v134Cp0004d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v134Cp0003d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v134Cp0002d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v134Cp0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v1234p0002d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v1234p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0EEFp0002d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0EEFp0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0123p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v3823p0002d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v3823p0001d*dc*dsc*dp*ic*isc*ip*in*
alias:          usb:v0EEFp0002d*dc*dsc*dp*ic03isc*ip*in*
alias:          usb:v0EEFp0001d*dc*dsc*dp*ic03isc*ip*in*
depends:
retpoline:      Y
intree:         Y
name:           usbtouchscreen
vermagic:       5.16.12-arch1-1 SMP preempt mod_unload
sig_id:         PKCS#7
signer:         Build time autogenerated kernel key
sig_key:        5B:91:A2:B2:2D:19:EA:F2:23:3D:57:51:3C:19:43:67:96:4B:B5:CB
sig_hashalgo:   sha512
signature:      30:66:02:31:00:DA:1F:9C:3C:B7:EB:10:F6:02:EA:38:64:0A:23:44:
		14:27:FD:CD:73:E2:58:DB:54:4A:61:BA:A3:88:2C:CD:0F:EC:15:87:
		77:5A:31:B4:37:B8:2F:04:A0:54:38:B3:EF:02:31:00:AB:2F:64:87:
		3D:83:2F:15:06:63:4B:D0:C5:C6:41:9C:C8:80:B7:13:B5:CE:F0:31:
		CD:15:8F:39:94:86:38:35:9A:9E:30:67:2E:55:70:79:E4:C9:77:2E:
		56:DB:82:F2
parm:           swap_xy:If set X and Y axes are swapped. (bool)
parm:           hwcalib_xy:If set hw-calibrated X/Y are used if available (bool)
filename:       /lib/modules/5.16.12-arch1-1/kernel/drivers/hid/usbhid/usbhid.ko.zst
license:        GPL
description:    USB HID core driver
author:         Jiri Kosina
author:         Vojtech Pavlik
author:         Andreas Gal
srcversion:     1E6640E0B624E5E24BB7031
alias:          usb:v*p*d*dc*dsc*dp*ic03isc*ip*in*
depends:
retpoline:      Y
intree:         Y
name:           usbhid
vermagic:       5.16.12-arch1-1 SMP preempt mod_unload
sig_id:         PKCS#7
signer:         Build time autogenerated kernel key
sig_key:        5B:91:A2:B2:2D:19:EA:F2:23:3D:57:51:3C:19:43:67:96:4B:B5:CB
sig_hashalgo:   sha512
signature:      30:64:02:30:06:91:8D:A6:21:87:93:51:E5:5E:EB:11:7C:55:7D:6F:
		3A:1E:58:A7:C3:74:AF:A0:0A:9E:A5:40:3D:58:84:02:8E:7A:D1:4C:
		20:97:2F:ED:3B:24:40:15:69:89:83:A8:02:30:31:15:DB:D1:1F:E0:
		C6:F6:38:F2:B6:E8:99:6A:0D:6D:3B:E4:47:CD:2C:B7:C1:9F:7B:46:
		8F:4E:EF:4D:33:CF:9A:34:4A:25:70:73:27:FD:9B:D7:E8:8F:5F:A5:
		00:CC
parm:           mousepoll:Polling interval of mice (uint)
parm:           jspoll:Polling interval of joysticks (uint)
parm:           kbpoll:Polling interval of keyboards (uint)
parm:           ignoreled:Autosuspend with active leds (uint)
parm:           quirks:Add/modify USB HID quirks by specifying  quirks=vendorID:productID:quirks where vendorID, productID, and quirks are all in 0x-prefixed hex (array of charp)
```

However, even if I do not blacklist the usbtouchscreen driver, the output of `usb-devices` is not changed. Maybe 1. the `usbhid` driver internally always delegates to the `usbtouchscreen` driver, or 2. the `usbtouchscreen` driver is the default but somehow does not work and `usbhid` is used instead.

When `usbtouchscreen` is not blacklisted and I insert the usb cable, I see the following dmesg output. So maybe it first tries `usbtouchscreen` and then falls back on `usbhid`?
```
[ 1513.347989] usb 3-2: new full-speed USB device number 5 using xhci_hcd
[ 1513.504914] usb 3-2: New USB device found, idVendor=0eef, idProduct=0001, bcdDevice= 1.00
[ 1513.504921] usb 3-2: New USB device strings: Mfr=1, Product=2, SerialNumber=0
[ 1513.504924] usb 3-2: Product: USB TouchController
[ 1513.504926] usb 3-2: Manufacturer: eGalax Inc.
[ 1514.076638] usbcore: registered new interface driver usbtouchscreen
[ 1514.087895] input: Logitech USB Optical Mouse as /devices/pci0000:00/0000:00:08.1/0000:07:00.4/usb5/5-2/5-2:1.0/0003:046D:C077.0004/input/input24
[ 1514.088116] hid-generic 0003:046D:C077.0004: input,hidraw0: USB HID v1.11 Mouse [Logitech USB Optical Mouse] on usb-0000:07:00.4-2/input0
[ 1514.092199] input: eGalax Inc. USB TouchController as /devices/pci0000:00/0000:00:08.1/0000:07:00.3/usb3/3-2/3-2:1.0/0003:0EEF:0001.0005/input/input25
[ 1514.092520] input: eGalax Inc. USB TouchController Touchscreen as /devices/pci0000:00/0000:00:08.1/0000:07:00.3/usb3/3-2/3-2:1.0/0003:0EEF:0001.0005/input/input26
[ 1514.092696] hid-generic 0003:0EEF:0001.0005: input,hidraw1: USB HID v2.10 Pointer [eGalax Inc. USB TouchController] on usb-0000:07:00.3-2/input0
[ 1514.092753] usbcore: registered new interface driver usbhid
[ 1514.092755] usbhid: USB HID core driver
```

[This random blogpost](http://handychen.blogspot.com/2011/03/try-egalaxy-usb-touch-for-tslib.html) says that `usbhid` and `usbtouchscreen` are in conflict anyways, maybe `usbtouchscreen` was superseeded at some point.

## Using eGTouchD

The touchscreen works if you just put the supplied xorg conf file in /usr/share/X11/xorg.conf.d and start the daemon with `eGTouchD start`.
The binary also just reads the raw input and uses uinput to create a virtual mouse.