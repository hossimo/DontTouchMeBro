using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Data;
using System.Diagnostics;
using System.Drawing;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace DontTouchMeBro
{
    public partial class AboutWindow : Form
    {
        public AboutWindow()
        {
            InitializeComponent();
        }

        public void SetDeviceID(string deviceID)
        {
            DeviceID_textBox.Text = deviceID;
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

        private void DeviceID_textBox_TextChanged(object sender, EventArgs e)
        {

        }
    }
}
