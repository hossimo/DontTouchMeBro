using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Text;

namespace DontTouchMeBro
{
    public static class ErrorLogger
    {
        private static readonly string LogPath = Path.Combine(
            Path.GetDirectoryName(Assembly.GetExecutingAssembly().Location),
            "error_log.txt");
            
        private static readonly object _lockObj = new object();
        
        public static void LogError(string context, Exception ex)
        {
            try
            {
                StringBuilder sb = new StringBuilder();
                sb.AppendLine($"[{DateTime.Now:yyyy-MM-dd HH:mm:ss}] ERROR in {context}");
                
                if (ex != null)
                {
                    sb.AppendLine($"Message: {ex.Message}");
                    sb.AppendLine($"Source: {ex.Source}");
                    sb.AppendLine($"Stack Trace: {ex.StackTrace}");
                    
                    if (ex.InnerException != null)
                    {
                        sb.AppendLine($"Inner Exception: {ex.InnerException.Message}");
                        sb.AppendLine($"Inner Stack Trace: {ex.InnerException.StackTrace}");
                    }
                }
                else
                {
                    sb.AppendLine("No exception details available");
                }
                
                sb.AppendLine(new string('-', 50));
                
                lock (_lockObj)
                {
                    File.AppendAllText(LogPath, sb.ToString());
                }
                
                Debug.WriteLine($"Error logged: {context} - {ex?.Message ?? "No exception details"}");
            }
            catch (Exception logEx)
            {
                // Last resort if logging fails
                Debug.WriteLine($"Failed to log error: {logEx.Message}");
            }
        }
        
        public static void LogInfo(string message)
        {
            try
            {
                string logEntry = $"[{DateTime.Now:yyyy-MM-dd HH:mm:ss}] INFO: {message}{Environment.NewLine}";
                
                lock (_lockObj)
                {
                    File.AppendAllText(LogPath, logEntry);
                }
                
                Debug.WriteLine(message);
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"Failed to log info: {ex.Message}");
            }
        }
    }
}
