using System;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Threading;
using System.Windows.Forms;

namespace DontTouchMeBro
{
    internal static class Program
    {
        static NotifyIcon trayIcon;

        static readonly Icon iconYes = Properties.Resources.YesIcon;
        static readonly Icon iconNo = Properties.Resources.NoIcon;
        static string instanceID = "";
        static string path = Path.Combine(Directory.GetCurrentDirectory(), "device-id.txt");

        private static Mutex mutex = null;

        [STAThread]
        static void Main()
        {
            bool createdNew = false;
            instanceID = ReadConfigFile(path);

            mutex = new Mutex(true, "DontTouchMeBro!", out createdNew);

            if (!createdNew)
            {
                Debug.WriteLine("Exitting, Already Running.");
            }

            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);

            ContextMenu trayMenu = new ContextMenu();
            trayMenu.MenuItems.Add("Reveal in File Explorer", OnShowSettings);
            trayMenu.MenuItems.Add("Configure", OnShowAbout);
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
            mutex.ReleaseMutex();

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
            else if (result.Contains("No devices were found on the system."))
            {
                MessageBox.Show($"Could not find a device with the name\n\n{instanceID}\n\nPlease check the instance name and restart.", "Device Not Found");
                return;
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
                CreateNoWindow = true,
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

        static void OnShowSettings(object sender, EventArgs e)
        {
            string strExeFilePath = System.Reflection.Assembly.GetExecutingAssembly().Location;
            string strWorkPath = System.IO.Path.GetDirectoryName(strExeFilePath);

            ProcessStartInfo start = new ProcessStartInfo
            {
                Arguments = strWorkPath,
                FileName = "explorer.exe"
            };
            Process.Start(start);
        }
        static void OnShowAbout(object sender, EventArgs e)
        {
            AboutWindow aboutWindow = new AboutWindow();
            aboutWindow.SetDeviceID(instanceID);
            aboutWindow.ShowDialog();
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

        static public void SetDeviceID(string deviceID)
        {
            instanceID= deviceID;
            WriteConfigFile(path, deviceID.Trim());
        }

        static string ReadConfigFile(string path)
        {
            string result = null;
           try
            {
                result = File.ReadAllText(path).Trim();
            }
            catch (Exception)
            {
                MessageBox.Show($"Please meake a text file at\n{path}\n with the content of the Device you want to control.", "Missing config file");
            }

            return result;
        }
        static void WriteConfigFile(string path, string deviceID)
        {
            try
            {
                File.WriteAllText(path, deviceID);
            }
            catch (Exception)
            {
                MessageBox.Show($"Could not write to {path}", "Could not write file");
                throw;
            }

        }
    }
}
