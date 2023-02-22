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
        static DeviceManager.DeviceItem CurrentDevice;
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

            // Application Stuff
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);

            // Setup Menues
            ContextMenu trayMenu = new ContextMenu();
            trayMenu.MenuItems.Add("Reveal in File Explorer", OnShowSettings);
            trayMenu.MenuItems.Add("Configure", OnShowAbout);
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

        // On Startup read the stored instanceID and check if the device is enabled or disabled.
        // TODO: If the device is null of not found, show the about window.
        static void OnStart()
        {

            CurrentDevice = DeviceManager.GetDeviceID(ReadConfigFile(path));
            Debug.WriteLine($"DEVICE CODE: {CurrentDevice.ConfigManagerErrorCode}");
            if (CurrentDevice.ConfigManagerErrorCode == "0")
            {
                SetDeviceIcon(true);
            }
            else if (CurrentDevice.ConfigManagerErrorCode == "22")
            {
                SetDeviceIcon(false);
            }
            else
            {
                Debug.WriteLine($"Device Code not handled: {CurrentDevice.ConfigManagerErrorCode}");
            }
        }

        static void OnClick(object sender, EventArgs e)
        {
            MouseEventArgs mouseArgs = (MouseEventArgs)e;

            // handle right click by ignoring it.
            if (mouseArgs.Button == MouseButtons.Right)
            {
                return;
            }

            // toggle device based on icon state
            if (trayIcon.Icon == iconYes)
            {
                DeviceManager.DisableDevice(CurrentDevice.id);
                SetDeviceIcon(false);
            }
            else
            {
                DeviceManager.EnableDevice(CurrentDevice.id);
                SetDeviceIcon(true);
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
            AboutWindow aboutWindow = new AboutWindow();

            aboutWindow.ShowDialog();
        }

        static void SetDeviceIcon(bool enabled)
        {
            if (enabled)
            {
                trayIcon.Icon = iconYes;
                trayIcon.Text = $"{CurrentDevice.description} Enabled";
            }
            else
            {
                trayIcon.Icon = iconNo;
                trayIcon.Text = $"{CurrentDevice.description} Disabled";
            }
        }

        static public void SetDeviceID(string deviceID)
        {
            CurrentDevice = DeviceManager.GetDeviceID(deviceID);
            WriteConfigFile(path, deviceID);
            SetDeviceIcon(DeviceManager.IsDeviceEnabled(deviceID));
            Debug.WriteLine($"Wrote Device ID: {deviceID} to config {path}.");
        }

        static public string GetDeviceID()
        {
            return CurrentDevice.id;
        }

        static public DeviceManager.DeviceItem GetCurrentDevice()
        {
            return CurrentDevice;
        }

        static string ReadConfigFile(string path)
        {
            string result = null;
            try
            {
                result = File.ReadAllText(path).Trim();
                Debug.WriteLine($"Read Device ID: {result} from config {path}.");
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
