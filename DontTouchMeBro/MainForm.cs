using System;
using System.Diagnostics;
using System.Drawing;
using System.Windows.Forms;

namespace DontTouchMeBro
{
    public class MainForm : Form
    {
        NotifyIcon _notifyIcon;
        private readonly uint _taskbarCreatedMessage;

        readonly Icon iconYes = Properties.Resources.YesIcon;
        readonly Icon iconNo = Properties.Resources.NoIcon;
        readonly Icon iconError = Properties.Resources.ErrorIcon;

        public MainForm()
        {
            _taskbarCreatedMessage = NativeMethods.RegisterTaskbarCreatedMessage();
            _notifyIcon = CreateNotifyIcon();
        }

        // Builds the tray icon and its context menu.
        private NotifyIcon CreateNotifyIcon()
        {
            ContextMenuStrip trayMenu = new ContextMenuStrip();
            // I don't like how this depends on Program, really need to setup a better messaging model
            trayMenu.Items.Add("Reveal in File Explorer", null, Program.OnShowSettings);
            trayMenu.Items.Add("Configure", null, Program.OnShowAbout);
            trayMenu.Items.Add("Exit", null, Program.OnExit);

            NotifyIcon notifyIcon = new NotifyIcon
            {
                Text = "Dont Touch Me Bro",
                ContextMenuStrip = trayMenu,
                Visible = true
            };

            notifyIcon.Click += OnClick;
            return notifyIcon;
        }

        // EVENTS

        // Once the window handle exists, let the (non-elevated) Explorer's
        // TaskbarCreated broadcast through UIPI to this window only.
        protected override void OnHandleCreated(EventArgs e)
        {
            base.OnHandleCreated(e);

            if (_taskbarCreatedMessage != 0)
            {
                NativeMethods.AllowTaskbarCreatedMessage(Handle, _taskbarCreatedMessage);
            }
        }

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
            if (_taskbarCreatedMessage != 0 && (uint)m.Msg == _taskbarCreatedMessage)
            {
                Debug.WriteLine("WM_TASKBARCREATED - Explorer restarted, restoring icon");
                RestoreNotifyIcon();
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
            try
            {
                if (_notifyIcon == null)
                {
                    ErrorLogger.LogError("SetDeviceIcon called with null _notifyIcon", null);
                    RecreateNotifyIcon();
                }
                
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
                
                // Ensure the icon is visible
                if (!_notifyIcon.Visible)
                {
                    _notifyIcon.Visible = true;
                }
            }
            catch (Exception ex)
            {
                ErrorLogger.LogError("Error setting device icon", ex);
            }
        }

        // Re-add the notify icon after Explorer restarts (TaskbarCreated)
        public void RestoreNotifyIcon()
        {
            try
            {
                if (_notifyIcon != null)
                {
                    // Toggling visibility re-adds the icon to the new taskbar
                    _notifyIcon.Visible = false;
                    _notifyIcon.Visible = true;

                    // Also make sure the icon itself is properly set
                    SetDeviceIcon(Program.GetCurrentDevice());
                    
                    Debug.WriteLine("Notify icon restored");
                }
                else
                {
                    RecreateNotifyIcon();
                }
            }
            catch (Exception ex)
            {
                ErrorLogger.LogError("Error restoring notify icon", ex);
            }
        }

        // Add this method to recreate the notify icon if it's completely lost
        private void RecreateNotifyIcon()
        {
            try
            {
                if (_notifyIcon != null)
                {
                    _notifyIcon.Dispose();
                }
                
                _notifyIcon = CreateNotifyIcon();

                // Make sure the icon is set properly
                SetDeviceIcon(Program.GetCurrentDevice());
                
                Debug.WriteLine("Notify icon recreated");
            }
            catch (Exception ex)
            {
                ErrorLogger.LogError("Error recreating notify icon", ex);
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