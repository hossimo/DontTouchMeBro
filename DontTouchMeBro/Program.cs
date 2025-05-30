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
        static readonly string path = Path.Combine(Directory.GetCurrentDirectory(), "device-id.txt");
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
            CurrentDevice = DeviceManager.GetDeviceID(ReadConfigFile(path));


            Debug.WriteLine($"DEVICE CODE: {CurrentDevice.ConfigManagerErrorCode}");
            if (CurrentDevice.ConfigManagerErrorCode == "0")
            {
                mainForm.SetDeviceIcon(CurrentDevice);
            }
            else if (CurrentDevice.ConfigManagerErrorCode == "22")
            {
                mainForm.SetDeviceIcon(CurrentDevice);
            }
            else
            {
                mainForm.SetDeviceIcon(CurrentDevice);
                Debug.WriteLine($"Device Code not handled: {CurrentDevice.ConfigManagerErrorCode}");
            }

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
            string strExeFilePath = System.Reflection.Assembly.GetExecutingAssembly().Location;
            string strWorkPath = Path.GetDirectoryName(strExeFilePath);

            ProcessStartInfo start = new ProcessStartInfo
            {
                Arguments = strWorkPath,
                FileName = "explorer.exe"
            };
            Process.Start(start);
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
                    File.WriteAllText(
                        Path.Combine(Directory.GetCurrentDirectory(), "fatal_error.log"),
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