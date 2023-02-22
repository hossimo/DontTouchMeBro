using System;
using System.Collections.Generic;
using System.Windows.Forms;

namespace DontTouchMeBro
{
    public partial class AboutWindow : Form
    {
        private ListViewItem lastChecked;

        public AboutWindow()
        {
            InitializeComponent();
            DeviceID_textBox.Text = Program.GetCurrentDevice().id;
            listView1.ItemCheck += ListView1_ItemCheck;

            version_label.Text = $"Version: {Application.ProductVersion}";

            // set the headers for the list view
            listView1.Columns.Add("Manufacturer", -2);
            listView1.Columns.Add("Description", -2);
            listView1.Columns.Add("ConfigManagerErrorCode", -2);
            listView1.Columns.Add("ID", 0);

            Populate_List_Box(DeviceManager.GetDeviceItems());
        }

        private void ListView1_ItemCheck(object sender, ItemCheckEventArgs e)
        {
            if (lastChecked != null && lastChecked.Checked && lastChecked != listView1.Items[e.Index])
                lastChecked.Checked = false;

            lastChecked = listView1.Items[e.Index];
            DeviceID_textBox.Text = lastChecked.SubItems[3].Text;
        }

        private void Populate_List_Box(List<DeviceManager.DeviceItem> devices)
        {
            listView1.Items.Clear();

            foreach (var device in devices)
            {
                ListViewItem listViewItem = new ListViewItem(device.manufacturer);
                listViewItem.SubItems.Add(device.description);
                listViewItem.SubItems.Add(device.ConfigManagerErrorCode);
                listViewItem.SubItems.Add(device.id);

                // if the device.id == DeviceID_textBox.Text then enable the checkbox

                if (device.id == Program.GetCurrentDevice().id)
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
        }

        private void OK_Button_Click(object sender, EventArgs e)
        {
            Program.SetDeviceID(DeviceID_textBox.Text);
            this.Close();
        }
    }
}