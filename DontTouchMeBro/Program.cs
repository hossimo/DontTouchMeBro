using System;
using System.Diagnostics;
using System.Drawing;
using System.Threading;
using System.Windows.Forms;

namespace DontTouchMeBro
{
    internal static class Program
    {
        static NotifyIcon trayIcon;

        static readonly Icon iconYes = Properties.Resources.YesIcon;
        static readonly Icon iconNo = Properties.Resources.NoIcon;
        static readonly string instanceID = @"HID\ELAN2D25&COL01\5&2B77D6B&0&0000";


        [STAThread]
        static void Main()
        {
            Mutex mutex = new Mutex(false, "DontTouchMeBro!");
            if (mutex.WaitOne(0, false))
            {
                Application.EnableVisualStyles();
                Application.SetCompatibleTextRenderingDefault(false);

                ContextMenu trayMenu = new ContextMenu();
                trayMenu.MenuItems.Add("Exit", OnExit);

                trayIcon = new NotifyIcon
                {
                    Text = "Dont Touch Me Bro",
                    ContextMenu = trayMenu,
                    Visible = true
                };

                trayIcon.Click += OnClick;
                OnStart();
                Application.Run();
            }
        }
        static void OnStart()
        {
            string result;

            using (Process p = new Process())
            {
                p.StartInfo.FileName = (@"C:\Windows\System32\pnputil.exe");
                p.StartInfo.Verb = "runas";
                p.StartInfo.UseShellExecute = false;
                p.StartInfo.CreateNoWindow = true;
                p.StartInfo.Arguments = $"/enum-devices /instanceid \"{instanceID}\"";
                p.StartInfo.RedirectStandardOutput = true;
                p.Start();
                p.WaitForExit();
                result = p.StandardOutput.ReadToEnd();
            }
            if (result.Contains("Started"))
            {
                SetDevice(true);
            }

            else
            {
                SetDevice(false);
            }
        }

        static void OnClick(object sender, EventArgs e)
        {
            MouseEventArgs mouseArgs = (MouseEventArgs)e;
            if(mouseArgs.Button == MouseButtons.Right)
            {
                return;
            }

            ProcessStartInfo startInfo = new ProcessStartInfo(@"C:\Windows\System32\pnputil.exe")
            {
                Verb = "runas",
                UseShellExecute = false,
                CreateNoWindow = true
            };


            if (trayIcon.Icon == iconYes)
            {
                startInfo.Arguments = $"/disable-device \"{instanceID}\"";
                SetDevice(false);
            }
            else
            {
                startInfo.Arguments = $"/enable-device \"{instanceID}\"";
                SetDevice(true);
            }
            Process.Start(startInfo);
        }

        static void OnExit(object sender, EventArgs e)
        {
            trayIcon.Dispose();
            Application.Exit();
        }

        static void SetDevice(bool enabled)
        {
            if (enabled)
            {
                trayIcon.Icon = iconYes;
                trayIcon.Text = "Device Enabled";
            }
            else
            {
                trayIcon.Icon = iconNo;
                trayIcon.Text = "Device Disabled";
            }
        }
    }
}
