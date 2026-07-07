using System;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Windows.Forms;

namespace DontTouchMeBro
{
    internal static class Program
    {
        static public DeviceManager.DeviceItem CurrentDevice;

        // Config lives in %APPDATA%\DontTouchMeBro so it survives an install and
        // is found regardless of the process working directory (e.g. when
        // launched from a Start Menu shortcut).
        static readonly string ConfigDirectory = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            "DontTouchMeBro");
        static readonly string path = Path.Combine(ConfigDirectory, "device-id.txt");

        static MainForm mainForm;

        private static Mutex mutex = null;

        [STAThread]
        static void Main()
        {
            // Set up global exception handlers
            Application.ThreadException += new ThreadExceptionEventHandler(Application_ThreadException);
            AppDomain.CurrentDomain.UnhandledException += new UnhandledExceptionEventHandler(CurrentDomain_UnhandledException);
            
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


            mainForm = new MainForm();
            MigrateLegacyConfig();
            CurrentDevice = DeviceManager.GetDeviceID(ReadConfigFile(path));


            Debug.WriteLine($"DEVICE CODE: {CurrentDevice.ConfigManagerErrorCode}");
            // SetDeviceIcon already maps every ConfigManagerErrorCode (including
            // unknown ones) to the right icon, so a single call is enough.
            mainForm.SetDeviceIcon(CurrentDevice);

            Application.Run(mainForm);
            mutex.ReleaseMutex();
        }

        // EVENTS

        //OnExit
        public static void OnExit(object sender, EventArgs e)
        {
            //mainForm.DisposeIcon();
            Application.Exit();
        }

        //OnShowSettings
        public static void OnShowSettings(object sender, EventArgs e)
        {
            // Open the config directory where device-id.txt lives.
            Directory.CreateDirectory(ConfigDirectory);

            ProcessStartInfo start = new ProcessStartInfo
            {
                Arguments = ConfigDirectory,
                FileName = "explorer.exe"
            };
            Process.Start(start);
        }

        // One-time migration: earlier versions stored device-id.txt next to the
        // executable. If the new per-user config doesn't exist yet but a legacy
        // file does, copy it over so upgrades keep working.
        static void MigrateLegacyConfig()
        {
            try
            {
                if (File.Exists(path))
                {
                    return;
                }

                string legacyPath = Path.Combine(AppContext.BaseDirectory, "device-id.txt");
                if (File.Exists(legacyPath))
                {
                    Directory.CreateDirectory(ConfigDirectory);
                    File.Copy(legacyPath, path);
                    Debug.WriteLine($"Migrated legacy config from {legacyPath} to {path}.");
                }
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"Config migration failed: {ex.Message}");
            }
        }

        //OnShowAbout
        public static void OnShowAbout(object sender, EventArgs e)
        {
            AboutWindow aboutWindow = new AboutWindow();
            aboutWindow.SetDesktopLocation(Cursor.Position.X - aboutWindow.Width, Cursor.Position.Y - aboutWindow.Height);

            aboutWindow.ShowDialog();
        }

        //OnDeviceChange
        public static void SetDeviceID(string deviceID)
        {
            CurrentDevice = DeviceManager.GetDeviceID(deviceID);
            WriteConfigFile(path, deviceID);
            
            mainForm.SetDeviceIcon(CurrentDevice);
            Debug.WriteLine($"Wrote Device ID: {deviceID} to config {path}.");
        }

        public static string GetDeviceID()
        {
            return CurrentDevice.id;
        }

        public static DeviceManager.DeviceItem GetCurrentDevice()
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
                MessageBox.Show($"No device is configured yet.\n\nUse the tray icon's \"Configure\" option to pick a device, or create a text file at\n{path}\ncontaining the Device Instance Path you want to control.", "No device configured");
            }

            return result;
        }

        static void WriteConfigFile(string path, string deviceID)
        {
            try
            {
                Directory.CreateDirectory(Path.GetDirectoryName(path));
                File.WriteAllText(path, deviceID);
            }
            catch (Exception)
            {
                MessageBox.Show($"Could not write to {path}", "Could not write file");
                throw;
            }
        }
        
        private static void Application_ThreadException(object sender, ThreadExceptionEventArgs e)
        {
            try
            {
                Debug.WriteLine($"Thread Exception: {e.Exception.Message}");
                // Try to ensure the notify icon is visible
                if (mainForm != null)
                {
                    mainForm.RestoreNotifyIcon();
                }
            }
            catch (Exception ex)
            {
                // Last resort if even our error handler fails
                Debug.WriteLine($"Critical error in exception handler: {ex.Message}");
            }
        }

        private static void CurrentDomain_UnhandledException(object sender, UnhandledExceptionEventArgs e)
        {
            try
            {
                Exception ex = e.ExceptionObject as Exception;
                string errorMessage = ex?.ToString() ?? "Unknown error";
                Debug.WriteLine($"Unhandled Exception: {errorMessage}");
                
                // If this is a terminal exception, we can't recover
                if (e.IsTerminating)
                {
                    // Write to the config dir, which is guaranteed writable
                    // (the working directory may not be).
                    Directory.CreateDirectory(ConfigDirectory);
                    File.WriteAllText(
                        Path.Combine(ConfigDirectory, "fatal_error.log"),
                        $"Fatal error occurred at {DateTime.Now}: {errorMessage}"
                    );
                }
            }
            catch
            {
                // Nothing we can do here but try not to make things worse
            }
        }
    }
}