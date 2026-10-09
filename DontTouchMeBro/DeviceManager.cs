using System;
using System.Collections.Generic;
using System.Linq;
using System.Management;


namespace DontTouchMeBro
{
    public class DeviceManager
    {
        // Values of Win32_PnPEntity.ConfigManagerErrorCode (one name per code).
        // https://learn.microsoft.com/en-us/windows-hardware/drivers/install/device-manager-error-messages
        public struct ConfigManagerErrorCode
        {
            public const string OK = "0";
            public const string NOT_CONFIGURED = "1";
            public const string DRIVER_CORRUPTED = "3";
            public const string CANNOT_START = "10";
            public const string RESOURCE_CONFLICT = "12";
            public const string RESTART_REQUIRED = "14";
            public const string RESOURCES_UNIDENTIFIED = "16";
            public const string REINSTALL_DRIVERS = "18";
            public const string REGISTRY_ERROR = "19";
            public const string BEING_REMOVED = "21";
            public const string DISABLED = "22";
            public const string NOT_PRESENT = "24";
            public const string DRIVERS_NOT_INSTALLED = "28";
            public const string DISABLED_BY_FIRMWARE = "29";
            public const string DRIVER_LOAD_FAILED = "31";
            public const string STOPPED_REPORTED_PROBLEM = "43";
            public const string NOT_CONNECTED = "45";
        }

        public struct DeviceItem
        {
            public string id;
            public string description;
            public string manufacturer;
            public string ConfigManagerErrorCode;
        }

        // Outcome of an Enable/Disable call. ReturnValue is the WMI method's
        // return code (0 = success); Device is the device state re-read afterwards.
        public struct DeviceMethodResult
        {
            public uint ReturnValue;
            public DeviceItem Device;
        }

        const string SCOPE = "root\\CIMV2";

        // Short human-readable meaning of a ConfigManagerErrorCode.
        public static string DescribeErrorCode(string code)
        {
            switch (code)
            {
                case ConfigManagerErrorCode.OK: return "Working properly";
                case ConfigManagerErrorCode.NOT_CONFIGURED: return "Not configured correctly";
                case ConfigManagerErrorCode.DRIVER_CORRUPTED: return "Driver may be corrupted";
                case ConfigManagerErrorCode.CANNOT_START: return "Device cannot start";
                case ConfigManagerErrorCode.RESOURCE_CONFLICT: return "Resource conflict";
                case ConfigManagerErrorCode.RESTART_REQUIRED: return "Restart required";
                case ConfigManagerErrorCode.RESOURCES_UNIDENTIFIED: return "Cannot identify all resources";
                case ConfigManagerErrorCode.REINSTALL_DRIVERS: return "Drivers need reinstalling";
                case ConfigManagerErrorCode.REGISTRY_ERROR: return "Registry configuration damaged";
                case ConfigManagerErrorCode.BEING_REMOVED: return "Being removed";
                case ConfigManagerErrorCode.DISABLED: return "Disabled";
                case ConfigManagerErrorCode.NOT_PRESENT: return "Not present";
                case ConfigManagerErrorCode.DRIVERS_NOT_INSTALLED: return "Drivers not installed";
                case ConfigManagerErrorCode.DISABLED_BY_FIRMWARE: return "Disabled by firmware";
                case ConfigManagerErrorCode.DRIVER_LOAD_FAILED: return "Driver failed to load";
                case ConfigManagerErrorCode.STOPPED_REPORTED_PROBLEM: return "Stopped after reporting a problem";
                case ConfigManagerErrorCode.NOT_CONNECTED: return "Not connected";
                default: return $"Error code {code}";
            }
        }

        // Get ManagementObjectSearcher
        private static ManagementObjectSearcher GetManagementObjectSearcher()
        {
            const string QUERY = "SELECT * FROM Win32_PnPEntity WHERE PNPClass = 'HIDClass'";
            return new ManagementObjectSearcher(SCOPE, QUERY);
        }

        // Targeted query for a single device by ID (any device class). WQL string
        // comparison is case-insensitive, matching how instance IDs behave.
        private static ManagementObjectSearcher GetDeviceSearcher(string deviceID)
        {
            string escaped = deviceID.Replace("\\", "\\\\").Replace("'", "\\'");
            string query = "SELECT DeviceID, Description, Manufacturer, ConfigManagerErrorCode " +
                           $"FROM Win32_PnPEntity WHERE DeviceID = '{escaped}'";
            return new ManagementObjectSearcher(SCOPE, query);
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

        // Get Device by ID. Returns an empty DeviceItem (id == null) if the ID is
        // empty or no device matches.
        public static DeviceItem GetDeviceID(string deviceID)
        {
            DeviceItem deviceItem = new DeviceItem();
            if (string.IsNullOrWhiteSpace(deviceID))
            {
                return deviceItem;
            }

            using (ManagementObjectSearcher deviceSearcher = GetDeviceSearcher(deviceID.Trim()))
            using (ManagementObjectCollection results = deviceSearcher.Get())
            {
                foreach (ManagementObject item in results.Cast<ManagementObject>())
                {
                    using (item)
                    {
                        deviceItem = ToDeviceItem(item);
                        break;
                    }
                }
            }
            return deviceItem;
        }

        // Disable Device
        public static DeviceMethodResult DisableDevice(DeviceItem deviceID)
        {
            return InvokeDeviceMethod(deviceID.id, "Disable");
        }

        //Enable Device by deviceID
        public static DeviceMethodResult EnableDevice(DeviceItem deviceID)
        {
            return InvokeDeviceMethod(deviceID.id, "Enable");
        }

        // Invoke a WMI method ("Enable"/"Disable") on the device with the given id
        // and return the method's return code plus the device's refreshed state.
        // Throws ManagementException on WMI failure and InvalidOperationException
        // if the device can't be found; callers decide how to report that.
        private static DeviceMethodResult InvokeDeviceMethod(string deviceID, string methodName)
        {
            if (string.IsNullOrWhiteSpace(deviceID))
            {
                throw new InvalidOperationException("No device ID configured.");
            }

            using (ManagementObjectSearcher deviceSearcher = GetDeviceSearcher(deviceID.Trim()))
            using (ManagementObjectCollection results = deviceSearcher.Get())
            {
                foreach (ManagementObject item in results.Cast<ManagementObject>())
                {
                    using (item)
                    {
                        object returnValue = item.InvokeMethod(methodName, new object[0]);

                        // Re-read this same object rather than enumerating again.
                        item.Get();

                        return new DeviceMethodResult
                        {
                            ReturnValue = Convert.ToUInt32(returnValue),
                            Device = ToDeviceItem(item)
                        };
                    }
                }
            }

            throw new InvalidOperationException($"Device not found: {deviceID}");
        }
    }
}
