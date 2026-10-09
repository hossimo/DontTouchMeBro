using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace DontTouchMeBro
{
    public partial class AboutWindow : Form
    {
        // ListView column indexes (must match the order columns are added below).
        private const int COLUMN_MANUFACTURER = 0;
        private const int COLUMN_DESCRIPTION = 1;
        private const int COLUMN_CLASS = 2;
        private const int COLUMN_ERROR_CODE = 3;
        private const int COLUMN_ID = 4;

        // The Configure dialog currently open, if any. The tray menu stays
        // usable while a modal dialog is up, so this is used to bring the
        // existing dialog forward instead of opening (nesting) another one.
        private static AboutWindow openInstance;

        // The checked item, so checking another one can uncheck it.
        private ListViewItem lastChecked;

        // The device ID that OK will save. Null/empty means nothing selected.
        private string selectedDeviceId;

        // Set while checks are changed programmatically so ListView1_ItemCheck
        // ignores the resulting (re-entrant) events.
        private bool updatingChecks;

        public AboutWindow()
        {
            InitializeComponent();
            openInstance = this;
            Disposed += AboutWindow_Disposed;

            listView1.ItemCheck += ListView1_ItemCheck;

            version_label.Text = $"Version: {Application.ProductVersion}";

            // set the headers for the list view
            listView1.Columns.Add("Manufacturer", -2);
            listView1.Columns.Add("Description", -2);
            listView1.Columns.Add("Class", -2);
            listView1.Columns.Add("ConfigManagerErrorCode", -2);
            listView1.Columns.Add("ID", 0);

            SetSelectedDevice(Program.GetCurrentDevice().id);

            // The device list itself is loaded asynchronously in OnShown so the
            // WMI query doesn't block the UI thread.
        }

        // If a Configure dialog is already open, activate it and return true.
        public static bool ActivateOpenInstance()
        {
            AboutWindow window = openInstance;
            if (window == null || window.IsDisposed || !window.Visible)
                return false;

            if (window.WindowState == FormWindowState.Minimized)
                window.WindowState = FormWindowState.Normal;

            window.Activate();
            window.BringToFront();
            return true;
        }

        // Location for a window of the given size near the cursor (above and to
        // the left, i.e. next to a bottom-right tray), clamped so the window
        // stays inside the working area of the monitor the cursor is on. This
        // handles taskbars on any edge and multi-monitor layouts.
        public static Point GetLocationNear(Point cursor, Size size)
        {
            Point preferred = new Point(cursor.X - size.Width, cursor.Y - size.Height);
            return ClampToWorkingArea(preferred, size, Screen.FromPoint(cursor).WorkingArea);
        }

        private static Point ClampToWorkingArea(Point location, Size size, Rectangle workingArea)
        {
            // Apply the Left/Top bound last so that a window larger than the
            // working area keeps its title bar visible.
            int x = Math.Max(workingArea.Left, Math.Min(location.X, workingArea.Right - size.Width));
            int y = Math.Max(workingArea.Top, Math.Min(location.Y, workingArea.Bottom - size.Height));
            return new Point(x, y);
        }

        protected override void OnLoad(EventArgs e)
        {
            base.OnLoad(e);

            // The size can change after the caller positioned us (DPI/font
            // auto-scaling happens during load), so re-clamp to the screen.
            Location = ClampToWorkingArea(Location, Size, Screen.FromRectangle(Bounds).WorkingArea);
        }

        protected override void OnShown(EventArgs e)
        {
            base.OnShown(e);
            LoadDevicesAsync();
        }

        protected override void OnFormClosed(FormClosedEventArgs e)
        {
            if (openInstance == this)
                openInstance = null;

            base.OnFormClosed(e);
        }

        private void AboutWindow_Disposed(object sender, EventArgs e)
        {
            if (openInstance == this)
                openInstance = null;
        }

        private void ShowAllClasses_checkBox_CheckedChanged(object sender, EventArgs e)
        {
            // Ignore changes before the first load; OnShown will load the list.
            if (!Visible)
                return;

            LoadDevicesAsync();
        }

        // Query WMI on a worker thread, then populate the list on the UI thread.
        private async void LoadDevicesAsync()
        {
            bool showAll = showAllClasses_checkBox.Checked;

            // Prevent another load from starting while this one runs.
            showAllClasses_checkBox.Enabled = false;
            listView1.Enabled = false;
            ClearList();
            status_label.Text = "Loading devices…";
            UseWaitCursor = true;

            try
            {
                List<DeviceManager.DeviceItem> devices = await Task.Run(() =>
                    showAll ? DeviceManager.GetAllDeviceItems() : DeviceManager.GetDeviceItems());

                // The dialog may have been closed while the query ran.
                if (IsDisposed)
                    return;

                Populate_List_Box(devices, showAll);
                status_label.Text = $"{devices.Count} device(s)";
            }
            catch (Exception ex)
            {
                if (IsDisposed)
                    return;

                ErrorLogger.LogError("AboutWindow.LoadDevicesAsync", ex);
                status_label.Text = "Failed to load devices.";
                MessageBox.Show(this, $"Failed to load the device list:\n\n{ex.Message}",
                    Text, MessageBoxButtons.OK, MessageBoxIcon.Error);
            }
            finally
            {
                if (!IsDisposed)
                {
                    UseWaitCursor = false;
                    listView1.Enabled = true;
                    showAllClasses_checkBox.Enabled = true;
                }
            }
        }

        private void ClearList()
        {
            listView1.Items.Clear();
            lastChecked = null;
        }

        private void ListView1_ItemCheck(object sender, ItemCheckEventArgs e)
        {
            if (updatingChecks)
                return;

            ListViewItem item = listView1.Items[e.Index];

            if (e.NewValue == CheckState.Checked)
            {
                // Only one device can be selected: uncheck the previous one.
                // Suppress the nested ItemCheck that this raises.
                if (lastChecked != null && lastChecked != item && lastChecked.Checked)
                {
                    updatingChecks = true;
                    try
                    {
                        lastChecked.Checked = false;
                    }
                    finally
                    {
                        updatingChecks = false;
                    }
                }

                lastChecked = item;
                SetSelectedDevice(item.SubItems[COLUMN_ID].Text);
            }
            else if (item == lastChecked)
            {
                // Unchecking the selected device clears the selection (and
                // disables OK). Cancel still keeps the configured device.
                lastChecked = null;
                SetSelectedDevice(null);
            }
        }

        private void SetSelectedDevice(string deviceId)
        {
            selectedDeviceId = deviceId;
            DeviceID_textBox.Text = deviceId ?? string.Empty;
            OK_Button.Enabled = !string.IsNullOrWhiteSpace(deviceId);
        }

        private void Populate_List_Box(List<DeviceManager.DeviceItem> devices, bool showAll)
        {
            IEnumerable<DeviceManager.DeviceItem> ordered = devices;
            if (showAll)
            {
                // Group the (long) all-classes list by class to make it scannable.
                ordered = devices
                    .OrderBy(d => d.pnpClass ?? string.Empty, StringComparer.OrdinalIgnoreCase)
                    .ThenBy(d => d.description ?? string.Empty, StringComparer.OrdinalIgnoreCase);
            }

            listView1.BeginUpdate();
            updatingChecks = true;
            try
            {
                ClearList();

                foreach (var device in ordered)
                {
                    string[] columns = new string[5];
                    columns[COLUMN_MANUFACTURER] = device.manufacturer;
                    columns[COLUMN_DESCRIPTION] = device.description;
                    // The HID-only query is filtered to HIDClass but doesn't
                    // return the class, so fill it in for that list.
                    columns[COLUMN_CLASS] = device.pnpClass ?? (showAll ? string.Empty : "HIDClass");
                    columns[COLUMN_ERROR_CODE] = device.ConfigManagerErrorCode;
                    columns[COLUMN_ID] = device.id;

                    ListViewItem listViewItem = new ListViewItem(columns);

                    // Check the item matching the current selection.
                    if (lastChecked == null && !string.IsNullOrEmpty(device.id) && device.id == selectedDeviceId)
                    {
                        listViewItem.Checked = true;
                        lastChecked = listViewItem;
                    }

                    listView1.Items.Add(listViewItem);
                }
            }
            finally
            {
                updatingChecks = false;
                listView1.EndUpdate();
            }

            lastChecked?.EnsureVisible();
        }

        private void OK_Button_Click(object sender, EventArgs e)
        {
            // Never save an empty ID (OK is disabled in that case too).
            if (string.IsNullOrWhiteSpace(selectedDeviceId))
                return;

            Program.SetDeviceID(selectedDeviceId);
            this.Close();
        }
    }
}
