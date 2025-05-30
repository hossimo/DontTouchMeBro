using System;
using System.Diagnostics;
using System.Runtime.InteropServices;

namespace DontTouchMeBro
{
    class NativeMethods
    {
        public const int WM_DISPLAYCHANGE = 0x007E;
        public const int WM_TASKBARCREATED = 0x0803;

        [DllImport("user32.dll", CharSet = CharSet.Auto)]
        public static extern IntPtr RegisterWindowMessage(string lpString);

        [DllImport("user32.dll", CharSet = CharSet.Auto)]
        public static extern bool ChangeWindowMessageFilter(uint message, uint dwFlag);

        // Add this method to ensure window messages are properly filtered
        public static void EnsureMessageFilters()
        {
            // Add message filter for TASKBARCREATED
            ChangeWindowMessageFilter(WM_TASKBARCREATED, 1);
            
            // Add message filter for DISPLAYCHANGE
            ChangeWindowMessageFilter(WM_DISPLAYCHANGE, 1);
            
            // These additional messages can help with shell integration
            ChangeWindowMessageFilter(0x0049, 1); // WM_COPYDATA
            ChangeWindowMessageFilter(0x0312, 1); // WM_HOTKEY
        }

        // Modify the RegisterTaskbarCreatedMessage method to use the improved filter
        public static IntPtr RegisterTaskbarCreatedMessage()
        {
            EnsureMessageFilters();
            
            IntPtr msgId = RegisterWindowMessage("TaskbarCreated");
            Debug.WriteLine($"Registered TaskbarCreated message: {msgId}");
            return msgId;
        }

        public static IntPtr RegisterDisplayChangeMessage()
        {
            bool result = ChangeWindowMessageFilter(WM_DISPLAYCHANGE, 1);
            Debug.WriteLineIf(result, "WM_DISPLAYCHANGE ADDED");

            return RegisterWindowMessage("DisplayChange");
        }
    }
}
