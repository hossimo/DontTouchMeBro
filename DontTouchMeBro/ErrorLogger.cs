using System;
using System.Diagnostics;
using System.Reflection;
using System.Text;

namespace DontTouchMeBro
{
    public static class ErrorLogger
    {
        private static readonly string EventSource = "DontTouchMeBroApp";
        private static readonly string LogName = "Application";
        private static readonly object _lockObj = new object(); // Make field readonly

        static ErrorLogger()
        {
            // Ensure the event source exists
            try
            {
                if (!EventLog.SourceExists(EventSource))
                {
                    EventLog.CreateEventSource(EventSource, LogName);
                }
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"Failed to create event source: {ex.Message}");
            }
        }

        public static void LogError(string context, Exception ex)
        {
            try
            {
                StringBuilder sb = new StringBuilder();
                sb.AppendLine($"ERROR in {context}");

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

                lock (_lockObj)
                {
                    EventLog.WriteEntry(EventSource, sb.ToString(), EventLogEntryType.Error);
                }

                Debug.WriteLine($"Error logged: {context} - {(ex != null ? ex.Message : "No exception details")}");
            }
            catch (Exception logEx)
            {
                Debug.WriteLine($"Failed to log error: {logEx.Message}");
            }
        }

        public static void LogInfo(string message)
        {
            try
            {
                string logEntry = $"INFO: {message}";

                lock (_lockObj)
                {
                    EventLog.WriteEntry(EventSource, logEntry, EventLogEntryType.Information);
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
