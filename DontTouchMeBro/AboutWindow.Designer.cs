using System.ComponentModel;
using System.Windows.Forms;

namespace DontTouchMeBro
{
    partial class AboutWindow
    {
        /// <summary>
        /// Required designer variable.
        /// </summary>
        private System.ComponentModel.IContainer components = null;

        /// <summary>
        /// Clean up any resources being used.
        /// </summary>
        /// <param name="disposing">true if managed resources should be disposed; otherwise, false.</param>
        protected override void Dispose(bool disposing)
        {
            if (disposing && (components != null))
            {
                components.Dispose();
            }
            base.Dispose(disposing);
        }

        AboutWindow(IContainer components, TextBox textBox1, Label label1, Label label2)
        {
            this.components = components;
            this.DeviceID_textBox = textBox1;
            this.DeviceId_Label = label1;
        }



        #region Windows Form Designer generated code

        /// <summary>
        /// Required method for Designer support - do not modify
        /// the contents of this method with the code editor.
        /// </summary>
        private void InitializeComponent()
        {
            System.Windows.Forms.FlowLayoutPanel flowLayoutPanel2;
            System.ComponentModel.ComponentResourceManager resources = new System.ComponentModel.ComponentResourceManager(typeof(AboutWindow));
            this.Cancel_button = new System.Windows.Forms.Button();
            this.OK_Button = new System.Windows.Forms.Button();
            this.DeviceID_textBox = new System.Windows.Forms.TextBox();
            this.DeviceId_Label = new System.Windows.Forms.Label();
            this.listView1 = new System.Windows.Forms.ListView();
            this.checkBox1 = new System.Windows.Forms.CheckBox();
            flowLayoutPanel2 = new System.Windows.Forms.FlowLayoutPanel();
            flowLayoutPanel2.SuspendLayout();
            this.SuspendLayout();
            // 
            // flowLayoutPanel2
            // 
            flowLayoutPanel2.Anchor = System.Windows.Forms.AnchorStyles.Bottom;
            flowLayoutPanel2.AutoSize = true;
            flowLayoutPanel2.AutoSizeMode = System.Windows.Forms.AutoSizeMode.GrowAndShrink;
            flowLayoutPanel2.Controls.Add(this.Cancel_button);
            flowLayoutPanel2.Controls.Add(this.OK_Button);
            flowLayoutPanel2.Location = new System.Drawing.Point(279, 318);
            flowLayoutPanel2.Margin = new System.Windows.Forms.Padding(2);
            flowLayoutPanel2.Name = "flowLayoutPanel2";
            flowLayoutPanel2.Size = new System.Drawing.Size(98, 27);
            flowLayoutPanel2.TabIndex = 6;
            // 
            // Cancel_button
            // 
            this.Cancel_button.AutoSize = true;
            this.Cancel_button.DialogResult = System.Windows.Forms.DialogResult.Cancel;
            this.Cancel_button.Location = new System.Drawing.Point(2, 2);
            this.Cancel_button.Margin = new System.Windows.Forms.Padding(2);
            this.Cancel_button.Name = "Cancel_button";
            this.Cancel_button.Size = new System.Drawing.Size(52, 23);
            this.Cancel_button.TabIndex = 4;
            this.Cancel_button.Text = "Cancel";
            this.Cancel_button.UseVisualStyleBackColor = true;
            this.Cancel_button.Click += new System.EventHandler(this.Cancel_button_Click);
            // 
            // OK_Button
            // 
            this.OK_Button.AutoSize = true;
            this.OK_Button.Location = new System.Drawing.Point(58, 2);
            this.OK_Button.Margin = new System.Windows.Forms.Padding(2);
            this.OK_Button.Name = "OK_Button";
            this.OK_Button.Size = new System.Drawing.Size(38, 23);
            this.OK_Button.TabIndex = 5;
            this.OK_Button.Text = "OK";
            this.OK_Button.UseVisualStyleBackColor = true;
            this.OK_Button.Click += new System.EventHandler(this.OK_Button_Click);
            // 
            // DeviceID_textBox
            // 
            this.DeviceID_textBox.Anchor = ((System.Windows.Forms.AnchorStyles)(((System.Windows.Forms.AnchorStyles.Top | System.Windows.Forms.AnchorStyles.Left) 
            | System.Windows.Forms.AnchorStyles.Right)));
            this.DeviceID_textBox.Location = new System.Drawing.Point(62, 6);
            this.DeviceID_textBox.Margin = new System.Windows.Forms.Padding(2);
            this.DeviceID_textBox.Name = "DeviceID_textBox";
            this.DeviceID_textBox.ReadOnly = true;
            this.DeviceID_textBox.Size = new System.Drawing.Size(579, 20);
            this.DeviceID_textBox.TabIndex = 0;
            // 
            // DeviceId_Label
            // 
            this.DeviceId_Label.AutoSize = true;
            this.DeviceId_Label.Location = new System.Drawing.Point(9, 9);
            this.DeviceId_Label.Margin = new System.Windows.Forms.Padding(2, 0, 2, 0);
            this.DeviceId_Label.Name = "DeviceId_Label";
            this.DeviceId_Label.Size = new System.Drawing.Size(55, 13);
            this.DeviceId_Label.TabIndex = 2;
            this.DeviceId_Label.Text = "Device ID";
            this.DeviceId_Label.Click += new System.EventHandler(this.OK_Button_Click);
            // 
            // listView1
            // 
            this.listView1.Anchor = ((System.Windows.Forms.AnchorStyles)((((System.Windows.Forms.AnchorStyles.Top | System.Windows.Forms.AnchorStyles.Bottom) 
            | System.Windows.Forms.AnchorStyles.Left) 
            | System.Windows.Forms.AnchorStyles.Right)));
            this.listView1.CheckBoxes = true;
            this.listView1.GridLines = true;
            this.listView1.HideSelection = false;
            this.listView1.Location = new System.Drawing.Point(12, 121);
            this.listView1.Name = "listView1";
            this.listView1.Size = new System.Drawing.Size(629, 192);
            this.listView1.TabIndex = 7;
            this.listView1.UseCompatibleStateImageBehavior = false;
            this.listView1.View = System.Windows.Forms.View.Details;
            // 
            // checkBox1
            // 
            this.checkBox1.AutoSize = true;
            this.checkBox1.Location = new System.Drawing.Point(12, 31);
            this.checkBox1.Name = "checkBox1";
            this.checkBox1.Size = new System.Drawing.Size(97, 17);
            this.checkBox1.TabIndex = 9;
            this.checkBox1.Text = "Show HID only";
            this.checkBox1.UseVisualStyleBackColor = true;
            this.checkBox1.CheckedChanged += new System.EventHandler(this.checkBox1_CheckedChanged);
            // 
            // AboutWindow
            // 
            this.AcceptButton = this.OK_Button;
            this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 13F);
            this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
            this.CancelButton = this.Cancel_button;
            this.ClientSize = new System.Drawing.Size(653, 351);
            this.ControlBox = false;
            this.Controls.Add(this.checkBox1);
            this.Controls.Add(this.DeviceId_Label);
            this.Controls.Add(this.listView1);
            this.Controls.Add(this.DeviceID_textBox);
            this.Controls.Add(flowLayoutPanel2);
            this.Icon = ((System.Drawing.Icon)(resources.GetObject("$this.Icon")));
            this.Margin = new System.Windows.Forms.Padding(2);
            this.MaximizeBox = false;
            this.MinimizeBox = false;
            this.Name = "AboutWindow";
            this.ShowInTaskbar = false;
            this.Text = "Dont Touch Me Bro!";
            flowLayoutPanel2.ResumeLayout(false);
            flowLayoutPanel2.PerformLayout();
            this.ResumeLayout(false);
            this.PerformLayout();

        }

        #endregion

        private System.Windows.Forms.TextBox DeviceID_textBox;
        private System.Windows.Forms.Label DeviceId_Label;
        private Button Cancel_button;
        private Button OK_Button;
        private ListView listView1;
        private CheckBox checkBox1;
    }
}