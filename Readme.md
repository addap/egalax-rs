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
$ sudo apt install libxrandr-dev libx11-dev libevdev-dev 
```

Then to build and install the program.
```
$ cargo build
$ cargo install --path .
```

TODO: check if the config files still work.

The `linux_config/` directory contains various configuration files to enable a smooth autostart for the driver. 
Put the file `egalax@.service` into `/etc/systemd/system` and `53-egalax.rules` into `/etc/udev/rules.d` to automatically start the driver when the monitor USB cable is plugged in.

## File Structure

- `dis/` - Ghidra project to disassemble the manufacturer's eGTouchD driver.
- `logs/` - Various log outputs which are discussed below.
- `Guide/` - Resources from the manufacturer. PDFs which describe the the monitor and a protocol of the touchscreen.  
           Though, our monitor actually uses a different protocol as discussed below.
- `linux_config/` - Config files to automatically start the driver when the USB cable is plugged in.
- `workspace` - Code for the driver.

## Background
At the student council we had an old iiyama ProLite T1930S monitor with an integrated touchscreen lying around which we didn't know how to use. There was probably a driver of the manufacturer that we could install, but wanting to learn more about Linux I decided to write our own userspace driver for it.

A *userspace driver* is a driver that runs as a normal user program, interacting with kernel APIs to implement the driver behavior. I decided on this approach as there is less danger of breaking things and we can use any language that can do system calls. 
Devices like mice, keyboards and touchscreens are collectively referred to as *human interface devices* (HID) and they are handled by the Linux [input subsystem](https://docs.kernel.org/input/input.html) which contains a generic usbhid driver that applies to many input devices. For userspace drivers the system exposes [uinput](https://docs.kernel.org/input/uinput.html) which allows creating and controlling virtual input devices.

HID devices (yes, like ATM machines, it just sounds better) communicate events (e.g. "button X was pressed") via binary messages called *HID reports* whose  schema is described by *HID report descriptors*.
In general, the usbhid driver's job is to convert HID reports into Linux input events.
It knows how to do this by first parsing the corresponding HID report descriptor for the device.

Since we want to write our own driver we use the kernel's [hidraw interface](https://docs.kernel.org/hid/hidraw.html) to get access to the original HID reports. 
Below is a summary of how a generic HID device with a kernel driver works compared to how we plan to treat the touchscreen.

<table>
<tr>
    <td><b>Generic USB Mouse</b></td>
    <td><b>Plan for egalax-rs</b></td>
</tr>
<tr>
    <td>

- Device sends HID reports to computer which are handled in the kernel by [usbhid](https://docs.kernel.org/input/input.html#hid-generic) and converted to input events.
- [evdev](https://docs.kernel.org/input/input.html#evdev) is the interface for userspace applications to receive input events. All the event nodes in `/dev/input/` belong to evdev.
- The xorg drivers `xf86-input-{evdev,libinput}` are wrappers around evdev to relay input events to the X server.
- Finally, the event reaches the X server and then the client application that will react to it.
    </td>
<td>

- Device sends HID reports to computer as before but instead of relying on usbhid in the kernel we get the raw HID report data in userspace via the [hidraw driver](https://docs.kernel.org/hid/hidraw.html).
- We interpret the HID report and generate input events that we inject back into the input subsystem using uinput.
- Then, evdev will present these events to userspace drivers as before.
- The xorg drivers `xf86-input-{evdev,libinput}` are wrappers around evdev to relay input events to the X server.
- Finally, the event reaches the X server and then the client application that will react to it.
</td>
</tr>
<tr>
    <td>

```
  X server & client                   
    ▲                                 
    │                                 
  xf86-input-{evdev,libinput}         
    ▲                                 
    │evdev                            
    │                       user-space
 ───┼─────────────────────────────────
    │                     kernel-space
    │                  input subsystem
  hidraw driver                       
    ▲                                 
    │                                 
  physical device                     
```

</td>
<td>

```
 X server & client                   
   ▲                                 
   │                                 
 xf86-input-{evdev,libinput}         
                    ▲                
 egalax-rs─┐        │evdev           
   ▲       │uinput  │      user-space            
───┼───────▼────────┼────────────────
   │                     kernel-space
   │                  input subsystem
 hidraw driver                       
   ▲                                 
   │                                 
 physical device                     
```
</td>
</tr>
</table>

Apart from writing the program itself, we need to clarify some points to carry out the plan:

1. Ensure that the usbhid driver does not try to handle the touchscreen, which could lead to duplicate input events.
2. Get access to the hidraw device.
3. Analyze the binary protocol of the touchscreen.

### Ensuring that we do not get Duplicate Input Events from the usbhid Driver

It turns out that some part of the Linux kernel can already handle the touchscreen and generate evdev events for it, but in the default configuration the X server was not picking them up.

We can add an xorg configuration file (put `linux_config/53-egalax-usbhid.conf` into `/etc/X11/xorg.conf.d/`), to command the X server to retrieve these events via evdev.
This will actually cause the mouse cursor to move but the movement is completely wrong when multiple monitors are active as the input is "stretched" over the entire virtual screen area.
Setting `--map-to-output` as described in the [arch wiki](https://wiki.archlinux.org/title/Touchscreen) might help.
But the movement is also very choppy and I wasn't able to generate a right-click, so I am not going to use this driver.

I am not sure what part of the kernel actually does the processing.
It is probably [`usbtouchscreen.c`](https://github.com/torvalds/linux/blob/a86bf2283d2c9769205407e2b54777c03d012939/drivers/input/touchscreen/usbtouchscreen.c) since others like [`egalax_ts.c`](https://github.com/torvalds/linux/blob/a86bf2283d2c9769205407e2b54777c03d012939/drivers/input/touchscreen/egalax_ts.c) and [`egalax_ts_serial.c`](https://github.com/torvalds/linux/blob/a86bf2283d2c9769205407e2b54777c03d012939/drivers/input/touchscreen/egalax_ts_serial.c) seem to handle eGalax touchscreens only for non-USB connections. 
So I just deleted the xorg configuration file again so that the generated events do not reach the X server.

### Getting Access to the Hidraw Device
The [hidraw](https://docs.kernel.org/hid/hidraw.html) documentation mentions the following.
```
Hidraw uses a dynamic major number, meaning that udev should be relied on to create hidraw device nodes.
```

This affected me because some time ago I used to use the first hidraw device `/dev/hidraw0` to read the touchscreen input, but currently this device is taken up by the buttons on my external USB speakers.
For development it's easier if we have a static device node. 
For that we use the following udev rules in the file `linux_config/51-hidraw-dev.rules`
```
SUBSYSTEM=="hidraw", ACTION=="add", SUBSYSTEMS=="usb", ATTRS{idProduct}=="0001", ATTRS{idVendor}=="0eef", GROUP="input", SYMLINK+="hidraw.egalax"
```
When the touchscreen is plugged-in this creates the device node `/dev/hidraw.egalax` from which we can read the HID reports.

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
First touching and releasing the upper-left corner, and then touching and releasing the upper-right corner of the monitor results in the following output.

```
$ xxd -b /dev/hidraw.egalax
00000000: 00000010 00000011 00111011 00000001 00110010 00000001  ..;.2.
00000006: 00000010 00000011 00111001 00000001 00110010 00000001  ..9.2.
0000000c: 00000010 00000011 00111000 00000001 00110001 00000001  ..8.1.
00000012: 00000010 00000011 00111000 00000001 00110011 00000001  ..8.3.
00000018: 00000010 00000011 00110110 00000001 00110110 00000001  ..6.6.
0000001e: 00000010 00000011 00110101 00000001 00111001 00000001  ..5.9.
00000024: 00000010 00000010 00110101 00000001 00111001 00000001  ..5.9.
0000002a: 00000010 00000011 01000101 00000001 10101100 00001110  ..E...
00000030: 00000010 00000011 01000100 00000001 10101100 00001110  ..D...
00000036: 00000010 00000011 01000011 00000001 10101100 00001110  ..C...
0000003c: 00000010 00000011 01000011 00000001 10101100 00001110  ..C...
00000042: 00000010 00000011 01000010 00000001 10101011 00001110  ..B...
00000048: 00000010 00000011 00111110 00000001 10101001 00001110  ..>...
0000004e: 00000010 00000011 00111010 00000001 10101000 00001110  ..:...
00000054: 00000010 00000011 00110111 00000001 10100111 00001110  ..7...
0000005a: 00000010 00000010 00110111 00000001 10100111 00001110  ..7...
```

Each HID report `m` consists of 6 bytes, which we number `m[0]` to `m[5]` from left to right. (However, bits are numbered from right to left.)
We can see the following pattern, where the first two bytes except for the touch flag are probably some constant metadata and padding.

```
m[0]        = 0x2 is constant
m[1][0]     = indicates whether we are touching or releasing a finger
m[1][7:1]   = 0b0000001, is constant

m[3] | m[2] = touch y position (little-endian)
m[5] | m[4] = touch x position (little-endian)
```

#### 2. Reading the HID report descriptor

Instead of crudely reading the actual HID reports we can also take a look at the HID report descriptors that describe their schema. 
This can be done using the `usbhid-dump` utitilty. 
We just need to point it to the correct device using the USB bus and device ID from above.

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

This series of hex bytes still doesn't really tell us anything though.
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

I'm still not sure what everything here means but apparently there are two kinds of reports, one being a "Pointer(Pointer)", the other a "Touch Screen(Stylus)". 
Both have 2 buttons with binary state (logical minimum = 0 & logical maximum = 1) that are represented by 1 bit each (report count = 2 & report size = 1). Then follow 6 bits of padding to complete a byte. Afterwards come the X and Y coordinates with 16 bits each (I assume the `Push` & `Pop` in the second report somehow apply the 16 bit report size to both).

In total this maps nicely to the values we see from the hidraw interface. 
The hidraw documentation tells us that "On a device which uses numbered reports, the first byte of the returned data will be the report number; the report data follows, beginning in the second byte."
That means the first byte `m[0]` that was a constant `0x2` designates the second type of HID reports for the touch screen interface.
The second byte `m[1]` includes two bits for the buttons and the rest is constant. Bytes `m[5:2]` then contain the X and Y value, although from my tests the Y position came first.

While the first `m[1][0]` as a "finger is touching" button is also what we observed, the second bit `m[1][1]` was a constant `1` and I am not aware of a second button that triggers it.
The report descriptor mentions its usage as "In Range".
From analyzing the source code of the manufacturer's driver (below) my theory is that bits `m[1][2:1]` indicate the screen resolution, so bit `m[1][1]` should have also been declared as constant instead of a button. 

Also, the minimum and maximum values for X (min 30, max 4040) and Y (min 60, max 4035) from the report descriptor are not what I observed.
That could be due to some of the touchscreen lying under the plastic bezel or an insufficient default calibration.
From trying to touch the outermost part of the monitor, both values actually seem to range from ca. 300 to ca. 3750.
Using the reported min and max values therefore results in a wrong cursor position.
So manual calibration is unavoidable if we want to avoid magic numbers in our binary.

#### 3. Using the manufacturer's driver

So it turns out there was a [driver from the manufacturer](https://www.eeti.com/drivers_Linux.html) the whole time.
While reading the raw HID reports and the HID report descriptors gives us an idea of how to parse the data, we can get an even better idea by analyzing this driver.
We use Ghidra to reverse engineer some functions in their `eGTouchU` driver. It helps that they have extensive debug logs for each function entry and exit.

TODO describe the analysis 

To summarize:
1. The first HID report byte `0x2` corresponds to the normal kind of message expected 
from the touch screen.
1. The first bit of the second byte `m[1][0]` is the status bit for a touch event happening.
2. The next two bits of the second byte `m[1][2:1]` encode the *resolution* of the touch screen. For our monitor it is a constant `0b01`.
According to the "Software Programming Guide (page 5)" corresponds to 12 bits of resolution in the X and Y axes, which is what we observe.
3. Bytes `m[5:2]` are then the X and Y position (with a resolution of 12 bits) in little-endian byte order.

### Conclusion

Putting everything together we can write a simple program that reads from the `/dev/hidraw.egalax` device node and creates uinput events which actually move the cursor. 
I did not discuss the code at all here but I did add a lot of comments to make the source code easy to digest. 
Most of it is straightforward anyways, except maybe for the overengineered `units` module to define newtypes for numbers in X and Y dimensions, which statically prevents errors like adding an X and a Y coordinate (now who would do that?). 

## Appendix
### Log Dumps
I recorded some output from connecting the touchscreen & touch interactions.
In these examples the touchscreen was always assigned the node `/dev/hidraw.egalax` for raw events, and `/dev/input/event19` for the evdev events generated by usbhid.

1. #### `hidraw.bin`
Contains the HID reports that the touchscreen sends when a touch interaction happens.
The following is the result of touching the 4 corners of the screen.

`tee hidraw.bin < /dev/hidraw.egalax | hexdump -C`


2. #### `xxd.log`.
The binary data above visualized with xxd. 
As explained above, it's already possible to deduce the binary format from this. 

`xxd -b hidraw.bin > xxd.log`


3. #### `event19.bin`
The evdev events generated by `usbhid` in the default configuration.

`tee event19.bin < /dev/input/event19 | hexdump -C`

4. #### `recording.txt`
The evdev events as reported by `evemu-record`.

`evemu-record /dev/input/event19`

This shows the evdev events emitted by the input device, which are similar to those we create with our driver.