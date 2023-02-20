using System;
using System.Diagnostics;
using System.Windows.Forms;
using System.Management;
using System.Collections.Generic;
using System.Management.Instrumentation;

namespace DontTouchMeBro
{

    struct DeviceItem {
        public string id;
        public string description;
        public string manufacturer;
        public string ConfigManagerErrorCode;
    }

    public partial class AboutWindow : Form
    {
        private string CurrentDevice = "";
        private ListViewItem lastChecked;
        private bool Show_HID_ONLY = true;

        public AboutWindow(string id)
        {
            InitializeComponent();
            CurrentDevice = id;
            DeviceID_textBox.Text = CurrentDevice;
            listView1.ItemCheck += ListView1_ItemCheck;

            // set the headers for the list view
            listView1.Columns.Add("Manufacturer", -2);
            listView1.Columns.Add("Description", -2);
            listView1.Columns.Add("ConfigManagerErrorCode", -2);
            listView1.Columns.Add("ID", 0);

            GetDevices();
        }

        private void ListView1_ItemCheck(object sender, ItemCheckEventArgs e)
        {
            if (lastChecked != null && lastChecked.Checked && lastChecked != listView1.Items[e.Index])
                lastChecked.Checked = false;
           
            lastChecked = listView1.Items[e.Index];
            DeviceID_textBox.Text = lastChecked.SubItems[3].Text;
        }

        private void GetDevices()
        {
            List<DeviceItem> devices = new List<DeviceItem>();
            ManagementObjectSearcher deviceSearcher = new ManagementObjectSearcher(
                "root\\CIMV2",
                "SELECT * FROM Win32_PnPEntity WHERE PNPClass = 'HIDClass'");
            foreach (ManagementObject item in deviceSearcher.Get())
            {

                if (item["DeviceID"].ToString() == "HID\\ELAN2D25&COL01\\5&2B77D6B&0&0000")
                {
                    foreach (var item2 in item.Properties)
                    {
                        Debug.WriteLine(item2.Name + ":" + item2.Value);
                    }
                }

                if (Show_HID_ONLY && !item["DeviceID"].ToString().StartsWith("HID"))
                {
                    continue;
                }
                else
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
                
            }
            Populate_List_Box(devices);
        }

        private void Populate_List_Box(List<DeviceItem> devices)
        {
            listView1.Items.Clear();



            foreach (var device in devices)
            {
                ListViewItem listViewItem = new ListViewItem(device.manufacturer);
                listViewItem.SubItems.Add(device.description);
                listViewItem.SubItems.Add(device.ConfigManagerErrorCode);
                listViewItem.SubItems.Add(device.id);

                // if the device.id == DeviceID_textBox.Text then enable the checkbox

                if (device.id == DeviceID_textBox.Text)
                {
                    listViewItem.Checked = true;
                    lastChecked = listViewItem;
                }

                else
                    listViewItem.Checked = false;

                listView1.Items.Add(listViewItem);
            }
        }

        private void Cancel_button_Click(object sender, EventArgs e)
        {

            Debug.WriteLine("CANCEL");
        }

        private void OK_Button_Click(object sender, EventArgs e)
        {
            Program.SetDeviceID(DeviceID_textBox.Text);
            Debug.WriteLine("OK");
            this.Close();
        }

        private void checkBox1_CheckedChanged(object sender, EventArgs e)
        {
            Show_HID_ONLY = checkBox1.Checked;
            GetDevices();
        }
    }
}


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