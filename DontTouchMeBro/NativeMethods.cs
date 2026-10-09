using System;
using System.Diagnostics;
using System.Runtime.InteropServices;

namespace DontTouchMeBro
{
    class NativeMethods
    {
        public const uint MSGFLT_ALLOW = 1;

        // "TaskbarCreated" is a registered message: its ID is assigned at runtime and
        // must be obtained from RegisterWindowMessage, never hard-coded.
        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        public static extern uint RegisterWindowMessage(string lpString);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool ChangeWindowMessageFilterEx(IntPtr hwnd, uint message, uint action, IntPtr pChangeFilterStruct);

        public static uint RegisterTaskbarCreatedMessage()
        {
            uint msgId = RegisterWindowMessage("TaskbarCreated");
            Debug.WriteLine($"Registered TaskbarCreated message: {msgId}");
            return msgId;
        }

        // The app runs elevated, so UIPI drops TaskbarCreated broadcasts from the
        // non-elevated Explorer unless this specific window explicitly allows it.
        public static bool AllowTaskbarCreatedMessage(IntPtr hwnd, uint taskbarCreatedMessage)
        {
            bool result = ChangeWindowMessageFilterEx(hwnd, taskbarCreatedMessage, MSGFLT_ALLOW, IntPtr.Zero);
            Debug.WriteLineIf(!result, $"ChangeWindowMessageFilterEx failed: {Marshal.GetLastWin32Error()}");
            return result;
        }
    }
}
