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
            this.flowLayoutPanel1 = new System.Windows.Forms.FlowLayoutPanel();
            flowLayoutPanel2 = new System.Windows.Forms.FlowLayoutPanel();
            flowLayoutPanel2.SuspendLayout();
            this.flowLayoutPanel1.SuspendLayout();
            this.SuspendLayout();
            // 
            // flowLayoutPanel2
            // 
            flowLayoutPanel2.Anchor = System.Windows.Forms.AnchorStyles.Bottom;
            flowLayoutPanel2.AutoSize = true;
            flowLayoutPanel2.AutoSizeMode = System.Windows.Forms.AutoSizeMode.GrowAndShrink;
            flowLayoutPanel2.Controls.Add(this.Cancel_button);
            flowLayoutPanel2.Controls.Add(this.OK_Button);
            flowLayoutPanel2.Location = new System.Drawing.Point(215, 88);
            flowLayoutPanel2.Name = "flowLayoutPanel2";
            flowLayoutPanel2.Size = new System.Drawing.Size(191, 42);
            flowLayoutPanel2.TabIndex = 6;
            // 
            // Cancel_button
            // 
            this.Cancel_button.AutoSize = true;
            this.Cancel_button.DialogResult = System.Windows.Forms.DialogResult.Cancel;
            this.Cancel_button.Location = new System.Drawing.Point(3, 3);
            this.Cancel_button.Name = "Cancel_button";
            this.Cancel_button.Size = new System.Drawing.Size(104, 36);
            this.Cancel_button.TabIndex = 4;
            this.Cancel_button.Text = "Cancel";
            this.Cancel_button.UseVisualStyleBackColor = true;
            this.Cancel_button.Click += new System.EventHandler(this.Cancel_button_Click);
            // 
            // OK_Button
            // 
            this.OK_Button.AutoSize = true;
            this.OK_Button.Location = new System.Drawing.Point(113, 3);
            this.OK_Button.Name = "OK_Button";
            this.OK_Button.Size = new System.Drawing.Size(75, 36);
            this.OK_Button.TabIndex = 5;
            this.OK_Button.Text = "OK";
            this.OK_Button.UseVisualStyleBackColor = true;
            this.OK_Button.Click += new System.EventHandler(this.OK_Button_Click);
            // 
            // DeviceID_textBox
            // 
            this.DeviceID_textBox.Location = new System.Drawing.Point(113, 3);
            this.DeviceID_textBox.Name = "DeviceID_textBox";
            this.DeviceID_textBox.Size = new System.Drawing.Size(469, 31);
            this.DeviceID_textBox.TabIndex = 0;
            this.DeviceID_textBox.TextChanged += new System.EventHandler(this.DeviceID_textBox_TextChanged);
            // 
            // DeviceId_Label
            // 
            this.DeviceId_Label.AutoSize = true;
            this.DeviceId_Label.Location = new System.Drawing.Point(3, 0);
            this.DeviceId_Label.Name = "DeviceId_Label";
            this.DeviceId_Label.Size = new System.Drawing.Size(104, 25);
            this.DeviceId_Label.TabIndex = 2;
            this.DeviceId_Label.Text = "Device ID";
            this.DeviceId_Label.Click += new System.EventHandler(this.OK_Button_Click);
            // 
            // flowLayoutPanel1
            // 
            this.flowLayoutPanel1.Controls.Add(this.DeviceId_Label);
            this.flowLayoutPanel1.Controls.Add(this.DeviceID_textBox);
            this.flowLayoutPanel1.Dock = System.Windows.Forms.DockStyle.Top;
            this.flowLayoutPanel1.Location = new System.Drawing.Point(0, 0);
            this.flowLayoutPanel1.Name = "flowLayoutPanel1";
            this.flowLayoutPanel1.Size = new System.Drawing.Size(621, 69);
            this.flowLayoutPanel1.TabIndex = 3;
            // 
            // AboutWindow
            // 
            this.AcceptButton = this.OK_Button;
            this.AutoScaleDimensions = new System.Drawing.SizeF(12F, 25F);
            this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
            this.CancelButton = this.Cancel_button;
            this.ClientSize = new System.Drawing.Size(621, 142);
            this.ControlBox = false;
            this.Controls.Add(flowLayoutPanel2);
            this.Controls.Add(this.flowLayoutPanel1);
            this.Icon = ((System.Drawing.Icon)(resources.GetObject("$this.Icon")));
            this.MaximizeBox = false;
            this.MinimizeBox = false;
            this.Name = "AboutWindow";
            this.ShowInTaskbar = false;
            this.Text = "Dont Touch Me Bro!";
            flowLayoutPanel2.ResumeLayout(false);
            flowLayoutPanel2.PerformLayout();
            this.flowLayoutPanel1.ResumeLayout(false);
            this.flowLayoutPanel1.PerformLayout();
            this.ResumeLayout(false);
            this.PerformLayout();

        }

        #endregion

        private System.Windows.Forms.TextBox DeviceID_textBox;
        private System.Windows.Forms.Label DeviceId_Label;
        private FlowLayoutPanel flowLayoutPanel1;
        private Button Cancel_button;
        private Button OK_Button;
    }
}