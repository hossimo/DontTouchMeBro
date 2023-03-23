using System;
using System.Diagnostics;
using System.Drawing;
using System.Windows.Forms;

namespace DontTouchMeBro
{
    public class MainForm : Form
    {
        NotifyIcon _notifyIcon;
        private IntPtr _taskbarCreatedMessage;

        readonly Icon iconYes = Properties.Resources.YesIcon;
        readonly Icon iconNo = Properties.Resources.NoIcon;
        readonly Icon iconError = Properties.Resources.ErrorIcon;


        public MainForm()
        {
            ContextMenu trayMenu = new ContextMenu();
            // I don't like how this depends on Program, really need to setup a better messaging model
            trayMenu.MenuItems.Add("Reveal in File Explorer", Program.OnShowSettings);
            trayMenu.MenuItems.Add("Configure", Program.OnShowAbout);
            trayMenu.MenuItems.Add("Exit", Program.OnExit);

            _notifyIcon = new NotifyIcon
            {
                Text = "Dont Touch Me Bro",
                ContextMenu = trayMenu,
                Visible = true
            };

            _notifyIcon.Click += OnClick;

            _taskbarCreatedMessage = NativeMethods.RegisterTaskbarCreatedMessage();
        }

        // EVENTS

        // Set the main windows to hidden and minimized.
        // this window is only used to handle the tray icon an listen for events
        protected override void OnLoad(EventArgs e)
        {
            base.OnLoad(e);

            WindowState = FormWindowState.Minimized;
            ShowInTaskbar = false;
            Visible = false;
        }

        // LISTEN TO THE MESSAGE PUMP.
        protected override void WndProc(ref Message m)
        {
            switch (m.Msg)
            {
                case NativeMethods.WM_TASKBARCREATED:
                    Debug.WriteLine("WM_TASKBARCREATED");
                    _notifyIcon.Visible = false;
                    _notifyIcon.Visible = true;
                    break;
                default:
                    break;
            }
            base.WndProc(ref m);
        }


        protected override void Dispose(bool disposing)
        {
            if (disposing)
            {
                if (_notifyIcon != null)
                {
                    _notifyIcon.Dispose();
                    _notifyIcon = null;
                }
            }

            base.Dispose(disposing);
        }

        public void SetDeviceIcon(DeviceManager.DeviceItem deviceItem)
        {
            switch (deviceItem.ConfigManagerErrorCode)
            {
                case "0":
                    _notifyIcon.Icon = iconYes;
                    _notifyIcon.Text = $"Don't Touch Me Bro - {deviceItem.description} Enabled";
                    break;
                case "22":
                    _notifyIcon.Icon = iconNo;
                    _notifyIcon.Text = $"Don't Touch Me Bro - {deviceItem.description} Disabled";
                    break;
                default:
                    _notifyIcon.Icon = iconError;
                    _notifyIcon.Text = "Don't Touch Me Bro - Device ID Not Found";
                    break;
            }
        }


        // events
        void OnClick(object sender, EventArgs e)
        {
            MouseEventArgs mouseArgs = (MouseEventArgs)e;

            // handle right click by ignoring it.
            if (mouseArgs.Button == MouseButtons.Right)
            {
                return;
            }

            // toggle device based on icon state
            if (_notifyIcon.Icon == iconYes)
            {
                DeviceManager.DisableDevice(Program.CurrentDevice); // dont like how this is bound to Program
                SetDeviceIcon(Program.CurrentDevice);
            }
            else
            {
                DeviceManager.EnableDevice(Program.CurrentDevice); // dont like how this is bound to Program
                SetDeviceIcon(Program.CurrentDevice);
            }
        }

    }
}