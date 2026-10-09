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

        // The device ID from the config file. Kept separately from CurrentDevice
        // because CurrentDevice.id is null while the device isn't present.
        static string configuredDeviceID;

        // Re-reads the device state periodically so the icon follows changes made
        // elsewhere (Device Manager, unplug, sleep). See OnStateTimerTick.
        static System.Windows.Forms.Timer stateTimer;
        const int StatePollIntervalMs = 10000;

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
            Application.SetHighDpiMode(HighDpiMode.PerMonitorV2);
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);


            mainForm = new MainForm();
            MigrateLegacyConfig();
            configuredDeviceID = ReadConfigFile(path);
            CurrentDevice = QueryDevice(configuredDeviceID);


            Debug.WriteLine($"DEVICE CODE: {CurrentDevice.ConfigManagerErrorCode}");
            // SetDeviceIcon already maps every ConfigManagerErrorCode (including
            // unknown ones) to the right icon, so a single call is enough.
            mainForm.SetDeviceIcon(CurrentDevice);

            // A UI-thread timer (rather than a WMI event watcher) keeps this simple:
            // no cross-thread marshalling, and one targeted query every few seconds
            // is cheap.
            stateTimer = new System.Windows.Forms.Timer { Interval = StatePollIntervalMs };
            stateTimer.Tick += OnStateTimerTick;
            stateTimer.Start();

            try
            {
                Application.Run(mainForm);
            }
            finally
            {
                stateTimer.Dispose();
                mutex.ReleaseMutex();
            }
        }

        // Query the device, logging and returning an empty item on WMI failure.
        static DeviceManager.DeviceItem QueryDevice(string deviceID)
        {
            try
            {
                return DeviceManager.GetDeviceID(deviceID);
            }
            catch (Exception ex)
            {
                ErrorLogger.LogError($"Querying device {deviceID}", ex);
                return new DeviceManager.DeviceItem();
            }
        }

        // Refresh the icon if the device's state changed outside this app.
        static void OnStateTimerTick(object sender, EventArgs e)
        {
            if (string.IsNullOrEmpty(configuredDeviceID))
            {
                return;
            }

            DeviceManager.DeviceItem latest;
            try
            {
                latest = DeviceManager.GetDeviceID(configuredDeviceID);
            }
            catch (Exception ex)
            {
                // Don't spam the Event Log every poll; transient WMI errors are common.
                Debug.WriteLine($"State poll failed: {ex.Message}");
                return;
            }

            if (latest.id != CurrentDevice.id || latest.ConfigManagerErrorCode != CurrentDevice.ConfigManagerErrorCode)
            {
                Debug.WriteLine($"Device state changed: {CurrentDevice.ConfigManagerErrorCode} -> {latest.ConfigManagerErrorCode}");
                CurrentDevice = latest;
                mainForm.SetDeviceIcon(CurrentDevice);
            }
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
            // The tray menu stays usable while a modal dialog is open, so this
            // can be re-entered. Bring the existing dialog forward instead of
            // nesting a second one.
            if (AboutWindow.ActivateOpenInstance())
                return;

            // Modal forms are not disposed automatically when closed.
            using (AboutWindow aboutWindow = new AboutWindow())
            {
                aboutWindow.StartPosition = FormStartPosition.Manual;
                aboutWindow.Location = AboutWindow.GetLocationNear(Cursor.Position, aboutWindow.Size);

                aboutWindow.ShowDialog();
            }
        }

        //OnDeviceChange
        // Save the new device ID, and only switch to it once it's on disk.
        // Returns false (after telling the user) if the config couldn't be written.
        public static bool SetDeviceID(string deviceID)
        {
            deviceID = deviceID?.Trim() ?? string.Empty;

            if (!WriteConfigFile(path, deviceID))
            {
                return false;
            }
            Debug.WriteLine($"Wrote Device ID: {deviceID} to config {path}.");

            configuredDeviceID = deviceID;
            CurrentDevice = QueryDevice(deviceID);
            mainForm.SetDeviceIcon(CurrentDevice);
            return true;
        }

        // The device ID from the config file (may be null/empty if not configured).
        public static string GetConfiguredDeviceID()
        {
            return configuredDeviceID;
        }

        public static DeviceManager.DeviceItem GetCurrentDevice()
        {
            return CurrentDevice;
        }

        // Returns the configured device ID, or null if there isn't one (missing or
        // empty file) or it couldn't be read.
        static string ReadConfigFile(string path)
        {
            string result = null;
            try
            {
                result = File.ReadAllText(path).Trim();
                Debug.WriteLine($"Read Device ID: {result} from config {path}.");
            }
            catch (Exception ex) when (ex is FileNotFoundException || ex is DirectoryNotFoundException)
            {
                result = null;
            }
            catch (Exception ex)
            {
                ErrorLogger.LogError($"Reading config {path}", ex);
                MessageBox.Show($"Could not read the device configuration at\n{path}\n\n{ex.Message}", "Could not read config");
                return null;
            }

            if (string.IsNullOrEmpty(result))
            {
                MessageBox.Show($"No device is configured yet.\n\nUse the tray icon's \"Configure\" option to pick a device, or create a text file at\n{path}\ncontaining the Device Instance Path you want to control.", "No device configured");
                return null;
            }

            return result;
        }

        // Returns false (after telling the user) if the file couldn't be written.
        static bool WriteConfigFile(string path, string deviceID)
        {
            try
            {
                Directory.CreateDirectory(Path.GetDirectoryName(path));
                File.WriteAllText(path, deviceID);
                return true;
            }
            catch (Exception ex)
            {
                ErrorLogger.LogError($"Writing config {path}", ex);
                MessageBox.Show($"Could not write to {path}\n\n{ex.Message}", "Could not write file");
                return false;
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