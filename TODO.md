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
