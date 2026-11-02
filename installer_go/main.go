package main

import (
	"bufio"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

func findProjectRoot() string {
	wd, err := os.Getwd()
	if err != nil {
		return "."
	}

	for {
		if _, err := os.Stat(filepath.Join(wd, "Cargo.toml")); err == nil {
			if _, err := os.Stat(filepath.Join(wd, "scripts")); err == nil {
				return wd
			}
		}
		parent := filepath.Dir(wd)
		if parent == wd {
			break
		}
		wd = parent
	}

	return "."
}

func findPython() string {
	for _, candidate := range []string{"python3", "python"} {
		if _, err := exec.LookPath(candidate); err == nil {
			return candidate
		}
	}
	return "python"
}

func getDriveItems() []string {
	items := []string{"multiverse_disk.img (Virtual 64MB Image)"}
	if _, err := exec.LookPath("powershell"); err == nil {
		cmd := exec.Command("powershell", "-Command", "Get-Disk | Select-Object Number, FriendlyName, Size")
		out, err := cmd.Output()
		if err == nil {
			for _, line := range strings.Split(string(out), "\n") {
				line = strings.TrimSpace(line)
				if line == "" || strings.Contains(line, "----") || strings.Contains(line, "Number") {
					continue
				}
				items = append(items, "Physical Disk: "+line)
			}
		}
	}
	return items
}

func appendLog(log *widget.Entry, message string) {
	if log == nil {
		return
	}
	msg := strings.TrimSpace(message)
	if msg == "" {
		return
	}
	text := log.Text
	if text == "" {
		log.SetText(msg)
		return
	}
	log.SetText(text + "\n" + msg)
}

func ensureIsoDirectory(root string) string {
	isoDir := filepath.Join(root, "iso")
	if err := os.MkdirAll(isoDir, 0o755); err != nil {
		panic(err)
	}
	return isoDir
}

func runBuild(root string, log *widget.Entry) {
	python := findPython()
	cmd := exec.Command(python, filepath.Join("scripts", "make_bootable_img.py"))
	cmd.Dir = root
	appendLog(log, "[+] Building bootable disk image...")
	out, err := cmd.CombinedOutput()
	if err != nil {
		appendLog(log, fmt.Sprintf("[-] Build failed: %s", strings.TrimSpace(string(out))))
		return
	}
	appendLog(log, strings.TrimSpace(string(out)))
	appendLog(log, "[+] Build complete. Image ready to boot.")
}

func runQEMU(root string, log *widget.Entry) {
	python := findPython()
	cmd := exec.Command(python, filepath.Join("scripts", "run_qemu.py"))
	cmd.Dir = root
	appendLog(log, "[+] Launching QEMU...")
	out, err := cmd.CombinedOutput()
	if err != nil {
		appendLog(log, fmt.Sprintf("[-] QEMU failed: %s", strings.TrimSpace(string(out))))
		return
	}
	appendLog(log, strings.TrimSpace(string(out)))
}

func downloadFileWithHTTP(dstPath, sourceURL string) error {
	resp, err := http.Get(sourceURL)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("download failed with HTTP %s", resp.Status)
	}
	out, err := os.Create(dstPath)
	if err != nil {
		return err
	}
	defer out.Close()
	_, err = io.Copy(out, resp.Body)
	return err
}

func downloadWithTorrent(root, torrentSource string, log *widget.Entry) error {
	isoDir := ensureIsoDirectory(root)
	if strings.HasPrefix(torrentSource, "magnet:") {
		for _, tool := range []string{"aria2c", "transmission-cli", "transmission-remote"} {
			if _, err := exec.LookPath(tool); err == nil {
				cmd := exec.Command(tool, "-d", isoDir, torrentSource)
				cmd.Dir = root
				out, err := cmd.CombinedOutput()
				if err != nil {
					appendLog(log, fmt.Sprintf("[-] %s failed: %s", tool, strings.TrimSpace(string(out))))
					return err
				}
				appendLog(log, fmt.Sprintf("[%s] Torrent download started: %s", tool, torrentSource))
				appendLog(log, strings.TrimSpace(string(out)))
				return nil
			}
		}
		return fmt.Errorf("no torrent client installed; install aria2c or transmission-cli")
	}

	if strings.HasSuffix(strings.ToLower(torrentSource), ".torrent") {
		if _, err := os.Stat(torrentSource); err != nil {
			return err
		}
		for _, tool := range []string{"aria2c", "transmission-cli"} {
			if _, err := exec.LookPath(tool); err == nil {
				cmd := exec.Command(tool, "-d", isoDir, torrentSource)
				cmd.Dir = root
				out, err := cmd.CombinedOutput()
				if err != nil {
					appendLog(log, fmt.Sprintf("[-] %s failed: %s", tool, strings.TrimSpace(string(out))))
					return err
				}
				appendLog(log, strings.TrimSpace(string(out)))
				return nil
			}
		}
		return fmt.Errorf("torrent file provided but no supported client was found")
	}

	return fmt.Errorf("torrent input must be a magnet URL or a .torrent file")
}

func showHeader() {
	fmt.Printf("\033[2J\033[H")
	fmt.Println("╔══════════════════════════════════════════════════════╗")
	fmt.Println("║              Multiverse Installer TUI              ║")
	fmt.Println("╚══════════════════════════════════════════════════════╝")
}

func promptInput(label string) string {
	fmt.Print(label)
	reader := bufio.NewReader(os.Stdin)
	input, err := reader.ReadString('\n')
	if err != nil {
		return ""
	}
	return strings.TrimSpace(input)
}

func showLog(text string) {
	if text == "" {
		return
	}
	fmt.Println("\n---")
	fmt.Println(text)
	fmt.Println("---")
	fmt.Print("Press Enter to continue...")
	bufio.NewReader(os.Stdin).ReadString('\n')
}

func runTUI(root string) {
	for {
		showHeader()
		fmt.Println("1) Build bootable image")
		fmt.Println("2) Download ISO from URL")
		fmt.Println("3) Download torrent or magnet")
		fmt.Println("4) Load ISO from local disk")
		fmt.Println("5) Launch QEMU")
		fmt.Println("6) Exit")

		choice := promptInput("Select an option: ")
		switch choice {
		case "1":
			output, err := runCommand(root, "make_bootable_img.py")
			if err != nil {
				showLog("[ERROR] " + err.Error() + "\n" + output)
			} else {
				showLog(output)
			}
		case "2":
			url := promptInput("ISO URL: ")
			if url == "" {
				showLog("[ERROR] URL required.")
				continue
			}
			output, err := downloadDirectISO(root, url)
			if err != nil {
				showLog("[ERROR] " + err.Error())
			} else {
				showLog(output)
			}
		case "3":
			torrent := promptInput("Torrent file path or magnet URL: ")
			if torrent == "" {
				showLog("[ERROR] Torrent or magnet required.")
				continue
			}
			output, err := downloadWithTorrent2(root, torrent)
			if err != nil {
				showLog("[ERROR] " + err.Error() + "\n" + output)
			} else {
				showLog(output)
			}
		case "4":
			path := promptInput("ISO path: ")
			if path == "" {
				showLog("[ERROR] ISO path required.")
				continue
			}
			output, err := loadIsoFile2(root, path)
			if err != nil {
				showLog("[ERROR] " + err.Error())
			} else {
				showLog(output)
			}
		case "5":
			output, err := runCommand(root, "run_qemu.py")
			if err != nil {
				showLog("[ERROR] " + err.Error() + "\n" + output)
			} else {
				showLog(output)
			}
		case "6":
			fmt.Println("Goodbye.")
			return
		default:
			showLog("[ERROR] Invalid option. Choose 1-6.")
		}
	}
}

func runDirectGUI(root string) {
	app := app.New()
	win := app.NewWindow("Multiverse Installer")
	win.Resize(fyne.NewSize(780, 560))

	logBox := widget.NewEntry()
	logBox.MultiLine = true
	logBox.SetPlaceHolder("Welcome to Multiverse. Download or load an ISO, then build the image.\n")

	urlEntry := widget.NewEntry()
	urlEntry.SetPlaceHolder("https://example.com/ubuntu.iso")
	urlEntry.SetText("")

	torrentEntry := widget.NewEntry()
	torrentEntry.SetPlaceHolder("magnet:?xt=urn:btih:... or path/to/file.torrent")

	buildButton := widget.NewButtonWithIcon("Build Image", theme.DownloadIcon(), func() {
		go runBuild(root, logBox)
	})
	qemuButton := widget.NewButtonWithIcon("Launch QEMU", theme.MediaPlayIcon(), func() {
		go runQEMU(root, logBox)
	})
	buildButton.Importance = widget.HighImportance
	qemuButton.Importance = widget.SuccessImportance

	downloadButton := widget.NewButtonWithIcon("Download ISO", theme.DownloadIcon(), func() {
		go func() {
			selectedURL := strings.TrimSpace(urlEntry.Text)
			selectedTorrent := strings.TrimSpace(torrentEntry.Text)
			if selectedURL == "" && selectedTorrent == "" {
				appendLog(logBox, "[-] Enter a direct URL or a torrent/magnet source first.")
				return
			}
			if selectedTorrent != "" {
				if err := downloadWithTorrent(root, selectedTorrent, logBox); err != nil {
					appendLog(logBox, "[-] Torrent download failed: "+err.Error())
					return
				}
				return
			}
			isoDir := ensureIsoDirectory(root)
			filename := filepath.Base(parsedURLPath(selectedURL))
			if filename == "." || filename == "/" || filename == "" {
				filename = "downloaded.iso"
			}
			dest := filepath.Join(isoDir, filename)
			appendLog(logBox, "[+] Downloading ISO to "+dest)
			if err := downloadFileWithHTTP(dest, selectedURL); err != nil {
				appendLog(logBox, "[-] Download failed: "+err.Error())
				return
			}
			appendLog(logBox, "[+] Download complete: "+dest)
		}()
	})

	loadButton := widget.NewButtonWithIcon("Load ISO", theme.FolderOpenIcon(), func() {
		path := promptInput("ISO file path: ")
		if path == "" {
			appendLog(logBox, "[-] No ISO path entered.")
			return
		}
		if result, err := loadIsoFile(root, path, logBox); err != nil {
			appendLog(logBox, "[-] ISO load failed: "+err.Error())
		} else {
			appendLog(logBox, result)
		}
	})

	urlRow := container.New(layout.NewGridLayout(2), widget.NewLabel("ISO URL:"), urlEntry)
	torrentRow := container.New(layout.NewGridLayout(2), widget.NewLabel("Torrent / Magnet:"), torrentEntry)
	actionRow := container.New(layout.NewGridLayout(4), downloadButton, loadButton, buildButton, qemuButton)

	content := container.NewVBox(
		widget.NewLabelWithStyle("Multiverse Installer", fyne.TextAlignLeading, fyne.TextStyle{Bold: true}),
		widget.NewLabel("Download Linux or Windows ISOs, load local image files, and create a bootable multiverse disk."),
		urlRow,
		torrentRow,
		actionRow,
		widget.NewLabelWithStyle("Log", fyne.TextAlignLeading, fyne.TextStyle{Bold: true}),
		container.NewPadded(logBox),
	)

	card := widget.NewCard("ISO toolkit", "GUI mode for downloading and building images.", content)
	win.SetContent(container.NewPadded(card))
	win.ShowAndRun()
}

func downloadDirectISO(root, sourceURL string) (string, error) {
	isoDir := ensureIsoDirectory(root)
	filename := filepath.Base(parsedURLPath(sourceURL))
	if filename == "." || filename == "/" || filename == "" {
		filename = "downloaded.iso"
	}
	dest := filepath.Join(isoDir, filename)
	if err := downloadFileWithHTTP(dest, sourceURL); err != nil {
		return "", err
	}
	return "Downloaded to: " + dest, nil
}

func loadIsoFile2(root, sourcePath string) (string, error) {
	return loadIsoFile(root, sourcePath, nil)
}

func downloadWithTorrent2(root, source string) (string, error) {
	if err := downloadWithTorrent(root, source, nil); err != nil {
		return "", err
	}
	return "Torrent download started successfully.", nil
}

func loadIsoFile(root, sourcePath string, log *widget.Entry) (string, error) {
	if sourcePath == "" {
		return "", fmt.Errorf("no ISO file selected")
	}
	isoDir := ensureIsoDirectory(root)
	filename := filepath.Base(sourcePath)
	dest := filepath.Join(isoDir, filename)
	in, err := os.Open(sourcePath)
	if err != nil {
		return "", err
	}
	defer in.Close()
	out, err := os.Create(dest)
	if err != nil {
		return "", err
	}
	defer out.Close()
	if _, err = io.Copy(out, in); err != nil {
		return "", err
	}
	if log != nil {
		appendLog(log, "[+] ISO loaded to "+dest)
	}
	return "Loaded ISO to: " + dest, nil
}

func runCommand(root, script string) (string, error) {
	python := findPython()
	cmd := exec.Command(python, filepath.Join("scripts", script))
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	return strings.TrimSpace(string(out)), err
}

func main() {
	root := findProjectRoot()
	if len(os.Args) > 1 {
		switch os.Args[1] {
		case "--tui":
			runTUI(root)
			return
		case "--gui":
			runDirectGUI(root)
			return
		}
	}
	if os.Getenv("DISPLAY") != "" || os.Getenv("WAYLAND_DISPLAY") != "" {
		runDirectGUI(root)
		return
	}
	runTUI(root)
}

func parsedURLPath(raw string) string {
	parsed, err := url.Parse(raw)
	if err != nil || parsed.Path == "" {
		return filepath.Base(strings.TrimSpace(raw))
	}
	return filepath.Base(parsed.Path)
}
