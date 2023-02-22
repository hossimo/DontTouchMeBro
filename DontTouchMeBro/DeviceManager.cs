using System.Collections.Generic;
using System.Linq;
using System.Management;

// https://learn.microsoft.com/en-us/windows-hardware/drivers/install/device-manager-error-messages
//
// ConfigManagerErrorCode
//
//0: This value indicates that the device is working properly.
//1: This value indicates that the device is not configured correctly.
//2: This value indicates that the driver for the device is not installed.
//3: This value indicates that the driver for the device is not configured correctly.
//10: This value indicates that the device cannot start.
//12: This value indicates that the device has been disabled.
//14: This value indicates that the device has failed.
//16: This value indicates that the device is in a "not present" state.
//18: This value indicates that the device is not currently available.
//19: This value indicates that the device has no drivers installed.
//21: This value indicates that the device is not working properly due to a reconfiguration.
//22: This value indicates that the device is disabled.
//24: This value indicates that the device is not present, not working, or missing drivers.

namespace DontTouchMeBro
{
    internal class DeviceManager
    {
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

        // Get All Devices
        public static List<DeviceItem> GetDeviceItems()
        {
            List<DeviceItem> devices = new List<DeviceItem>();
            ManagementObjectSearcher deviceSearcher = GetManagementObjectSearcher();

            foreach (ManagementObject item in deviceSearcher.Get().Cast<ManagementObject>())
            {
                DeviceItem device = new DeviceItem
                {
                    id = item["DeviceID"].ToString(),
                    description = item["Description"].ToString(),
                    manufacturer = item["Manufacturer"].ToString(),
                    ConfigManagerErrorCode = item["ConfigManagerErrorCode"].ToString()
                };
                devices.Add(device);
            }
            return devices;
        }

        // Get Device by ID
        public static DeviceItem GetDeviceID(string deviceID)
        {
            DeviceItem deviceItem = new DeviceItem();
            ManagementObjectSearcher deviceSearcher = GetManagementObjectSearcher();

            foreach (ManagementObject item in deviceSearcher.Get().Cast<ManagementObject>())
            {
                if (item["DeviceID"].ToString() == deviceID)
                {
                    deviceItem.id = item["DeviceID"].ToString();
                    deviceItem.description = item["Description"].ToString();
                    deviceItem.manufacturer = item["Manufacturer"].ToString();
                    deviceItem.ConfigManagerErrorCode = item["ConfigManagerErrorCode"].ToString();
                    break;
                }
            }
            return deviceItem;
        }

        // Disable Device
        public static void DisableDevice(string deviceID)
        {
            ManagementObjectSearcher deviceSearcher = GetManagementObjectSearcher();

            foreach (ManagementObject item in deviceSearcher.Get().Cast<ManagementObject>())
            {
                if (item["DeviceID"].ToString() == deviceID)
                {
                    item.InvokeMethod("Disable", null, null);
                    break;
                }
            }
        }

        //Enable Device by deviceID
        public static void EnableDevice(string deviceID)
        {
            ManagementObjectSearcher deviceSearcher = GetManagementObjectSearcher();

            foreach (ManagementObject item in deviceSearcher.Get().Cast<ManagementObject>())
            {
                if (item["DeviceID"].ToString() == deviceID)
                {
                    item.InvokeMethod("Enable", null, null);
                    break;
                }
            }
        }

        public static bool IsDeviceEnabled(string deviceID)
        {
            ManagementObjectSearcher deviceSearcher = GetManagementObjectSearcher();

            foreach (ManagementObject item in deviceSearcher.Get().Cast<ManagementObject>())
            {
                if (item["DeviceID"].ToString() == deviceID)
                {
                    return item["ConfigManagerErrorCode"].ToString() == "0";
                }
            }
            return false;
        }
    }
}
