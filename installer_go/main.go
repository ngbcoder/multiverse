package main

import (
	"fmt"
	"os"
	"os/exec"
	"runtime/debug"
	"strings"

	"github.com/lxn/walk"
	. "github.com/lxn/walk/declarative"
)

const manifestContent = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<assemblyIdentity
    version="1.0.0.0"
    processorArchitecture="*"
    name="Multiverse.Installer"
    type="win32"
/>
<description>Multiverse Native Windows Installer</description>
<dependency>
    <dependentAssembly>
        <assemblyIdentity
            type="win32"
            name="Microsoft.Windows.Common-Controls"
            version="6.0.0.0"
            processorArchitecture="*"
            publicKeyToken="6595b64144ccf1df"
            language="*"
        />
    </dependentAssembly>
</dependency>
</assembly>`

func init() {
	// Ensure manifest file exists next to executable for Common Controls v6
	exePath, err := os.Executable()
	if err == nil {
		manifestPath := exePath + ".manifest"
		if _, err := os.Stat(manifestPath); os.IsNotExist(err) {
			_ = os.WriteFile(manifestPath, []byte(manifestContent), 0644)
		}
	}
}

type DriveItem struct {
	Name string
}

func getDriveItems() []DriveItem {
	items := []DriveItem{
		{Name: "multiverse_disk.img (Virtual 64MB Image)"},
	}

	cmd := exec.Command("powershell", "-Command", "Get-Disk | Select-Object Number, FriendlyName, Size")
	out, err := cmd.Output()
	if err == nil {
		lines := strings.Split(string(out), "\n")
		for _, line := range lines {
			line = strings.TrimSpace(line)
			if line != "" && !strings.Contains(line, "----") && !strings.Contains(line, "Number") {
				items = append(items, DriveItem{Name: "Physical Disk: " + line})
			}
		}
	}
	return items
}

func main() {
	defer func() {
		if r := recover(); r != nil {
			f, err := os.Create("crash.log")
			if err == nil {
				fmt.Fprintf(f, "PANIC: %v\n\nStack:\n%s", r, debug.Stack())
				f.Close()
			}
		}
	}()

	var mw *walk.MainWindow
	var logBox *walk.TextEdit
	var driveCb *walk.ComboBox

	drives := getDriveItems()

	_, err := MainWindow{
		AssignTo: &mw,
		Title:    "Multiverse Native Windows Installer",
		MinSize:  Size{Width: 650, Height: 450},
		Layout:   VBox{},
		Children: []Widget{
			Label{Text: "Select Target Storage Device:", Font: Font{Bold: true, PointSize: 11}},
			ComboBox{
				AssignTo:      &driveCb,
				Model:         drives,
				BindingMember: "Name",
				DisplayMember: "Name",
				CurrentIndex:  0,
			},
			Composite{
				Layout: HBox{Spacing: 10},
				Children: []Widget{
					PushButton{
						Text: "Install Platform",
						OnClicked: func() {
							logBox.AppendText("\r\n[+] Starting installation...\r\n")
							cmd := exec.Command("python", "scripts/make_bootable_img.py")
							out, err := cmd.CombinedOutput()
							if err != nil {
								logBox.AppendText(string(out) + "\r\n[-] Error: " + err.Error() + "\r\n")
							} else {
								logBox.AppendText(string(out) + "\r\n[+] Installation completed successfully!\r\n")
							}
						},
					},
					PushButton{
						Text: "Test in QEMU",
						OnClicked: func() {
							logBox.AppendText("\r\n[+] Launching QEMU...\r\n")
							cmd := exec.Command("python", "scripts/run_qemu.py")
							err := cmd.Start()
							if err != nil {
								logBox.AppendText("\r\n[-] Failed to start QEMU: " + err.Error() + "\r\n")
							} else {
								logBox.AppendText("\r\n[+] QEMU launched successfully.\r\n")
							}
						},
					},
				},
			},
			Label{Text: "Installation Log:", Font: Font{Bold: true, PointSize: 10}},
			TextEdit{
				AssignTo: &logBox,
				ReadOnly: true,
				VScroll:  true,
				Text:     "Welcome to the Multiverse Native Windows Installer.\r\nSelect a device above and click 'Install Platform'.\r\n",
			},
		},
	}.Run()

	if err != nil {
		f, _ := os.Create("crash.log")
		if f != nil {
			fmt.Fprintf(f, "MainWindow Run Error: %v\n", err)
			f.Close()
		}
	}
}
