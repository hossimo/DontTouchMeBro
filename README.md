# DontTouchMeBro

## What is this thing.
Very simple C# Application that enables and disables a device by its Device Instance Path.

## Why
Well I purchased a new Dell XPS 15 Laptop. and at the time to get the OLED screen you had to get a touch screen. I had no real need for the touch screen do I did a little digging and found that I could disable it by entering the command:
`pnputil /disable-device "HID\ELAN2D25&COL01\5&2B77D6B&0&0000"`

and re-enable it with:
`pnputil /enable-device "HID\ELAN2D25&COL01\5&2B77D6B&0&0000"`

For a while I had two shortcuts that ran batch scripts as Administrator (since you need admin access to enable or disable devices).

Then I got a little tired of that and made this really simple application that runs the above two commands based on the state of an icon in the Window Notification bar.

Job done...

## What's Next

Well Maybe I'll flush it out to ask the user for a Device instance path and store it in a file or registry, but to be honest it works for me at this point so that might take a while.

## How do I make it work my Device or Touch Screen

At this point you will need a text file in the same path as the executable called `device-id.txt` entering your specific Device instance path.


## How do I find my Device Instance Path?

There are a few ways to do this but the simplest seems to be:

* Hit Ctrl + X to and Choose `Device Manager`
* Find the device you are looking for, if it's a Touch Screen try in the Human Interface Devices
* Once you have found the device go it it's properties.
* Once in the properties, do to the Details Tab and under the Property choose Device instance path:
* In the value Section, copy this text and use it for the instanceID variable.

## Why does the application always popup a UAC (How User Account Control) Dialog?
Well in order for an application to make changes to any devices on the computer it needs to be an administrator. So I had a choice, make the application require administrator access to run, or make each press of the button require admin access, the former seemed less annoying to me.

## How can I get this app to run at startup without asking for UAC each time

Lots of options here but probably the easiest is to use Task Scheduler, something like [this](https://superuser.com/questions/770420/schedule-a-task-with-admin-privileges-without-a-user-prompt-in-windows-7)

## Hey, so if this asks for app ask for Admin rights can I trust it?
In a short, No; never blindly trust someone you don't know. I tried to do the right thing but you trust you.
