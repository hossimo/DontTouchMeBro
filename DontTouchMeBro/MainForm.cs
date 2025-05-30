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

        private System.Threading.Timer _watchdogTimer;
        private bool _isIconVisible = true;

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

            // Set up watchdog timer to check icon status every 30 seconds
            _watchdogTimer = new System.Threading.Timer(
                CheckNotifyIconStatus, 
                null, 
                TimeSpan.FromSeconds(30), 
                TimeSpan.FromSeconds(30));
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
            if (m.Msg == NativeMethods.WM_TASKBARCREATED)
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
                if (_watchdogTimer != null)
                {
                    _watchdogTimer.Dispose();
                    _watchdogTimer = null;
                }
                
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
                    _isIconVisible = true;
                }
            }
            catch (Exception ex)
            {
                ErrorLogger.LogError("Error setting device icon", ex);
            }
        }

        // Add this method to check and restore the notify icon
        private void CheckNotifyIconStatus(object state)
        {
            try
            {
                if (_notifyIcon == null)
                {
                    Debug.WriteLine("Watchdog detected null notify icon, recreating...");
                    RecreateNotifyIcon();
                    return;
                }

                // Check if the icon is visible (stored state doesn't match actual state)
                if (_isIconVisible && !_notifyIcon.Visible)
                {
                    Debug.WriteLine("Watchdog detected invisible icon, restoring...");
                    RestoreNotifyIcon();
                }
            }
            catch (Exception ex)
            {
                ErrorLogger.LogError("Watchdog timer error", ex);
            }
        }

        // Add this method to restore the notify icon
        public void RestoreNotifyIcon()
        {
            try
            {
                if (_notifyIcon != null)
                {
                    // Sometimes toggling visibility helps restore the icon
                    _notifyIcon.Visible = false;
                    System.Threading.Thread.Sleep(100);
                    _notifyIcon.Visible = true;
                    _isIconVisible = true;
                    
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
                
                ContextMenu trayMenu = new ContextMenu();
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
                _isIconVisible = true;
                
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