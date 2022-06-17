# egalax-rs

An input driver for our old egalax touchscreen. Translates the raw binary output from the screen to control a virtual mouse with uinput.

## Install
```bash
$ sudo apt install libudev-dev libxrandr-dev libx11-dev libevdev-dev libsdl2-dev libsdl2-mixer-dev libsdl2-image-dev libsdl2-ttf-dev libsdl2-gfx-dev
$ cargo build
$ cargo install --path .
```
Put the udev `.rules` files in `/etc/udev/rules.d` and the `egalax.service` unit file in `/etc/systemd/system/`.

## File Structure

- c_src/ - C files to test some libc/kernel APIs.
- dis/ - Ghidra project to disassemble the manufacturer's eGTouchD driver.
- dumps/ - Various log outputs which are discussed below.
- Guide/ - Resources from the manufacturer. PDFs which describe the the monitor and a raw binary protocol of the touchscreen. 
           Though, out monitor actually uses a different protocol as discussed below.
- linux_config/ - Config files to automatically start the driver when the USB cable is plugged in.
- media/ - Resources for the calibration tool.

### Output Dumps
I recorded some output from connecting the touchscreen & touch interactions.

- The raw output from `/dev/hidraw0` in `hidraw.bin`

 ```tee hidraw.bin < /dev/hidraw0 | hexdump -C```
 Contains the binary data that the touchscreen sends over USB when a touch interaction happens.

- The binary data above visualized in `xxd.log`. Result of touching the 4 corners of the screen.

```xxd -b /dev/hidraw0```
Already possible to deduce binary format from this. Further discussed below.

 - The evdev events generated by `usbhid` in `event19.bin`

 ```tee event19.bin < /dev/input/event19 | hexdump -C```

 - The evdev events as reported by `evemu-record` in recording.txt

 ```evemu-record /dev/input/event19```

AFAIU, the kernel already has a driver for touchscreens (usbhid) and it does generate an input device and exposes it in `/dev/event19`.
This shows the evdev events emitted by the input device, which are similar to those we create with our driver.
 
 - excerpts from `/var/log/Xorg.0.log` in `xorg-libinput.log` and `xorg-evdev.log`

 At first, I did not have the xorg evdev driver `xf86-input-evdev` installed. So after putting the `53-egalax-usbhid.conf` in `/usr/share/X11/xorg.conf.d` (and rebooting/restarting X) you can see in the log that it falls back on libinput, which in turn does not seem to like the touchscreen. The screen might just be too old for libinput.

 After installing the evdev driver it works, the touchscreen is registered in xinput and moves the cursor. However, it moves the cursor over the whole virtual screen space (all connected outputs combined) and is horribly calibrated, so we don't use it.

- The hid report descriptor in `hid-report-descriptor.txt`. 
(need https://github.com/DIGImend/hidrd/, and find out bus:device number from `lsusb`)

```$ sudo usbhid-dump -a 3:88 -p | tail -n +2 | xxd -r -p | opt/hidrd/src/hidrd-convert -o spec -```

HID devices use this to document the binary format they use. The relevant part for us seems to be the USAGE(X) and USAGE(Y) in lines 48 and 59, respectively. 
The minimum and maximum correspond to what the usbhid driver sets as can be gathered from `evemu-record`. 
Unfortunately, the screen reports wrong numbers (e.g. X values are supposed to range from 30 to 4040) while they actually range from 300 to 3700.
So manual calibration seems unavoidable if we want to avoid magic numbers in our binary.


## Binary Format
The format described in the "Software Programming Guide" does not exactly match what I'm seeing in the xxd output, but it's similar.

- Sends packets of 6 bytes, e.g. 
```
00000000: 00000010 00000011 00110010 00000001 01011001 00000001  ..2.Y.

0: always `0x02`. Analysis of the manufacturer's driver shows that this is the tag for touch event messages. We were not able to prosuce the other (control) messages yet.
1: metadata
  - bit 5:6 seem to indicate resolution as described in manual
    i.e. 0:1 => 12 bits of resolution
  - bit 7 is 1 iff finger is touching
2:3: 12 bits of y position
     the y value is (packet[3][4:] << 8) | packet[2]
4:5: 12 bits of x position
```

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

## Input stack
1. How one event travels from e.g. a connected mouse to an application
```
X client
    ^
    |
X server
    ^
    |
xf86-input-evdev|xf86-input-libinput
    ^
    |
 ~/dev/eventX~
    |
evdev
    ^
    |
kernel (usbhid module)
    ^
    |
physical device (via interrupt)
```

2. What we want to achieve 
```
X client
    ^
    |
X server
    ^
    |
xf86-input-evdev|xf86-input-libinput
     \ 
      \
rustGalax -------> uinput
    ^ |              |
    | |              |
 ~/dev/eventX~ <-----+
    |/                 
evdev 
    ^
    |
kernel (usbhid module/custom kernel module that just passes on events to userspace)
    ^
    |
physical device (via interrupt)
```


## TODO
- [x] use `usb-devices` while monitor is connected to check if it lists `usbhid`. Yes, so it does use the `usbhid` driver.
```
T:  Bus=03 Lev=01 Prnt=01 Port=01 Cnt=01 Dev#= 11 Spd=12  MxCh= 0
D:  Ver= 1.10 Cls=00(>ifc ) Sub=00 Prot=00 MxPS=64 #Cfgs=  1
P:  Vendor=0eef ProdID=0001 Rev=01.00
S:  Manufacturer=eGalax Inc.
S:  Product=USB TouchController
C:  #Ifs= 1 Cfg#= 1 Atr=a0 MxPwr=100mA
I:  If#= 0 Alt= 0 #EPs= 1 Cls=03(HID  ) Sub=00 Prot=00 Driver=usbhid
E:  Ad=81(I) Atr=03(Int.) MxPS=   8 Ivl=3ms
```
- [x] use lsof to check which process has the device file open, to check if it's the eGTouchD binary. As explained [here](https://unix.stackexchange.com/a/60080)

For a simple usb mouse it does not show any process that has the file open. 
- [x] follow [this article](https://who-t.blogspot.com/2016/09/understanding-evdev.html) and use evemu to list event node information for the egalax (For our virtual mouse/my touchpad it should show as in the article)
  - [ ] for a usb mouse it works as described. However, middle mouse click is not reported as an event in `/dev/input/eventX`, but it is in `/dev/input/mouse2`. Why does this happen?
  - [ ] for the touchscreen it also worked. But some event nodes associated with the touchscreen are "grabbed" as explained in [current status](#current-status). Need to find out what it means and who is grabbing it, probably xorg. `man evemu-record` tells you to use the `fuse` command but it gives no output for me.
- [x] use the xorg.conf file from the egalax driver without the eGTouchD binary

Does not work. The eGToucD binar creates a virtual input device with uinput, which the xorg.conf file matches against.
- [x] Find out the productname that usbhid gives the touchscreen [described here](https://unix.stackexchange.com/questions/58117/determine-xinput-device-manufacturer-and-model) and mofy the xotg.conf to use that. Maybe it already works then.

It does work, if I also install `xf86-input-evdev`. 

- [ ] With the custom xorg.conf, both the "touchcontroller" and the "touchcontroller touchscreen" are registered in xinput. Does it matter? By adding a `Driver "void"` stanza to our xorg.conf we could also ignore the "touchcontroller".

- [ ] try out calibration with [this](https://ubuntuforums.org/archive/index.php/t-1478877.html)
