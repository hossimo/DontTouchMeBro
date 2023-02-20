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
        static readonly string path = Path.Combine(Directory.GetCurrentDirectory(), "device-id.txt");

        private static Mutex mutex = null;

        [STAThread]
        static void Main()
        {
            // using mutex make sure that only one instance of the application is running.
            mutex = new Mutex(true, "DontTouchMeBro!", out bool createdNew);

            // check if the application is already running.
            if (!createdNew)
            {
                Debug.WriteLine("Exitting, Already Running.");
                return;
            }

            instanceID = ReadConfigFile(path);

            // Application Stuff
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);

            // Setup Menues
            ContextMenu trayMenu = new ContextMenu();
            trayMenu.MenuItems.Add("Reveal in File Explorer", OnShowSettings);
            //trayMenu.MenuItems.Add("Configure", OnShowAbout); //TODO: Hiding the menu until it works
            trayMenu.MenuItems.Add("Exit", OnExit);

            // Setup Tray Icon
            trayIcon = new NotifyIcon
            {
                Text = "Dont Touch Me Bro",
                ContextMenu = trayMenu,
                Visible = true
            };

            // events
            trayIcon.Click += OnClick;

            // Start the app
            OnStart();

            Application.Run();
            mutex.ReleaseMutex();

        }
        static void OnStart()
        {
            DeviceManager.DeviceItem deviceItem =  DeviceManager.GetDeviceID(instanceID);
            Debug.WriteLine($"DEVICE CODE: {deviceItem.ConfigManagerErrorCode}");
            if (deviceItem.ConfigManagerErrorCode == "0")
            {
                SetDevice(true);
            }
            else if (deviceItem.ConfigManagerErrorCode == "22")
            {
                SetDevice(false);
            }
            else
            {
                Debug.WriteLine($"Device Code not handled: {deviceItem.ConfigManagerErrorCode}");
            }
        }

        static void OnClick(object sender, EventArgs e)
        {
            MouseEventArgs mouseArgs = (MouseEventArgs)e;

            // handle right click by ignoring it.
            if(mouseArgs.Button == MouseButtons.Right)
            {
                return;
            }

            // toggle device based on icon state
            if (trayIcon.Icon == iconYes)
            {
                DeviceManager.DisableDevice(instanceID);
                SetDevice(false);
            }
            else
            {
                DeviceManager.EnableDevice(instanceID);
                SetDevice(true);
            }
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
            AboutWindow aboutWindow = new AboutWindow(instanceID);
            
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
