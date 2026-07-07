using System.Collections.Generic;
using System.Linq;
using System.Management;


namespace DontTouchMeBro
{
    public class DeviceManager
    {
        // https://learn.microsoft.com/en-us/windows-hardware/drivers/install/device-manager-error-messages
        public struct ConfigManagerErrorCode
        {
            public const string OK = "0";
            public const string NOT_CONFIGURED = "1";
            public const string DRIVER_NOT_INSTALLED = "2";
            public const string DRIVER_NOT_CONFIGURED = "3";
            public const string DEVICE_CANNOT_START = "10";
            public const string DEVICE_DISABLED = "12";
            public const string DEVICE_FAILED = "14";
            public const string DEVICE_NOT_PRESENT = "16";
            public const string DEVICE_NOT_AVAILABLE = "18";
            public const string DEVICE_NO_DRIVERS = "19";
            public const string DEVICE_RECONFIGURED = "21";
            public const string DEVICE_DISABLED2 = "22";
            public const string DEVICE_NOT_PRESENT2 = "24";
        }

        public struct DeviceItem
        {
            public string id;
            public string description;
            public string manufacturer;
            public string ConfigManagerErrorCode;
        }

        // Get ManagementObjectSearcher
        private static ManagementObjectSearcher GetManagementObjectSearcher()
        {
            const string SCOPE = "root\\CIMV2";
            const string QUERY = "SELECT * FROM Win32_PnPEntity WHERE PNPClass = 'HIDClass'";
            return new ManagementObjectSearcher(SCOPE, QUERY);
        }

        // Build a DeviceItem from a WMI management object.
        private static DeviceItem ToDeviceItem(ManagementObject item)
        {
            return new DeviceItem
            {
                id = item["DeviceID"]?.ToString(),
                description = item["Description"]?.ToString(),
                manufacturer = item["Manufacturer"]?.ToString(),
                ConfigManagerErrorCode = item["ConfigManagerErrorCode"]?.ToString()
            };
        }

        // Get All Devices
        public static List<DeviceItem> GetDeviceItems()
        {
            List<DeviceItem> devices = new List<DeviceItem>();

            using (ManagementObjectSearcher deviceSearcher = GetManagementObjectSearcher())
            using (ManagementObjectCollection results = deviceSearcher.Get())
            {
                foreach (ManagementObject item in results.Cast<ManagementObject>())
                {
                    using (item)
                    {
                        devices.Add(ToDeviceItem(item));
                    }
                }
            }
            return devices;
        }

        // Get Device by ID
        public static DeviceItem GetDeviceID(string deviceID)
        {
            DeviceItem deviceItem = new DeviceItem();

            using (ManagementObjectSearcher deviceSearcher = GetManagementObjectSearcher())
            using (ManagementObjectCollection results = deviceSearcher.Get())
            {
                foreach (ManagementObject item in results.Cast<ManagementObject>())
                {
                    using (item)
                    {
                        if (item["DeviceID"]?.ToString() == deviceID)
                        {
                            deviceItem = ToDeviceItem(item);
                            break;
                        }
                    }
                }
            }
            return deviceItem;
        }

        // Disable Device
        public static void DisableDevice(DeviceItem deviceID)
        {
            InvokeDeviceMethod(deviceID.id, "Disable");
        }

        //Enable Device by deviceID
        public static void EnableDevice(DeviceItem deviceID)
        {
            InvokeDeviceMethod(deviceID.id, "Enable");
        }

        // Invoke a WMI method ("Enable"/"Disable") on the device with the given id
        // and refresh Program.CurrentDevice from the same object.
        private static void InvokeDeviceMethod(string deviceID, string methodName)
        {
            using (ManagementObjectSearcher deviceSearcher = GetManagementObjectSearcher())
            using (ManagementObjectCollection results = deviceSearcher.Get())
            {
                foreach (ManagementObject item in results.Cast<ManagementObject>())
                {
                    using (item)
                    {
                        if (item["DeviceID"]?.ToString() == deviceID)
                        {
                            item.InvokeMethod(methodName, null, null);
                            Program.CurrentDevice = GetDeviceID(deviceID);
                            break;
                        }
                    }
                }
            }
        }

        public static bool IsDeviceEnabled(string deviceID)
        {
            using (ManagementObjectSearcher deviceSearcher = GetManagementObjectSearcher())
            using (ManagementObjectCollection results = deviceSearcher.Get())
            {
                foreach (ManagementObject item in results.Cast<ManagementObject>())
                {
                    using (item)
                    {
                        if (item["DeviceID"]?.ToString() == deviceID)
                        {
                            return item["ConfigManagerErrorCode"]?.ToString() == DeviceManager.ConfigManagerErrorCode.OK;
                        }
                    }
                }
            }
            return false;
        }
    }
}
