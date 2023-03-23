using System;
using System.Diagnostics;
using System.Runtime.InteropServices;

class NativeMethods
{
    public const int WM_DISPLAYCHANGE = 0x007E;
    public const int WM_TASKBARCREATED = 0x0803;

    [DllImport("user32.dll", CharSet = CharSet.Auto)]
    public static extern IntPtr RegisterWindowMessage(string lpString);

    [DllImport("user32.dll", CharSet = CharSet.Auto)]
    public static extern bool ChangeWindowMessageFilter(uint message, uint dwFlag);


    public static IntPtr RegisterTaskbarCreatedMessage()
    {
        bool result = ChangeWindowMessageFilter(WM_TASKBARCREATED, 1);
        Debug.WriteLineIf(result, "WM_TASKBARCREATED ADDED");

        return RegisterWindowMessage("TaskbarCreated");
    }

    public static IntPtr RegisterDisplayChangeMessage()
    {
        bool result = ChangeWindowMessageFilter(WM_DISPLAYCHANGE, 1);
        Debug.WriteLineIf(result, "WM_DISPLAYCHANGE ADDED");

        return RegisterWindowMessage("DisplayChange");
    }

}
