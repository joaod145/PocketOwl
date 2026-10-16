## Running the Host as a System Service

> [!IMPORTANT]
> This guide is specifically intended for **Debian-based systems using `systemd`**. The commands and service configuration may differ on other Linux distributions or init systems.

The PocketOwl host application can be configured as a `systemd` service so that the compiled Rust binary starts automatically during system startup.

### 1. Build the Rust Application

From the repository root:

```bash
cd host
cargo build --release
```

The compiled binary will normally be available at:

```bash
host/target/release/pocketowl
```

For a system-wide installation, it is recommended to place the application under `/opt`:

```bash
sudo mkdir -p /opt/pocketowl
sudo cp -r . /opt/pocketowl/host
```

The resulting structure should look similar to:

```text
/opt/pocketowl/
└── host/
    ├── Cargo.toml
    ├── Cargo.lock
    ├── src/
    └── target/
        └── release/
            └── pocketowl
```

### 2. Create a Dedicated User

For security reasons **(principle of least privilege)**, the service should not normally run as root.

Create a dedicated system user:

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin pocketowl
```

Then assign ownership of the application:

```bash
sudo chown -R pocketowl:pocketowl /opt/pocketowl
```

### 3. Create the systemd Service

Create the service definition:

```bash
sudo nano /etc/systemd/system/pocketowl.service
```

Add the following configuration:

```text
[Unit]
Description=PocketOwl System Monitor # description of the service
After=network-online.target # start PocketOwl after the network is considered online
Wants=network-online.target # requests that systemd also brings up the network-online.target dependency when starting PocketOwl

[Service]
Type=simple # tells systemd that the application runs as a normal foreground process

User=pocketowl
Group=pocketowl

ExecStart=/opt/pocketowl/host/target/release/pocketowl # specifies the compiled Rust executable that systemd should launch
WorkingDirectory=/opt/pocketowl/host # sets the application's working directory

Restart=always # tells systemd to automatically restart PocketOwl whenever the process terminates
RestartSec=3 # waits 3 seconds before attempting to restart the application

StandardOutput=journal # sends the application's standard output (stdout) to the systemd journal
StandardError=journal # sends the application's standard error to the same journal

[Install]
WantedBy=multi-user.target # specifies that the service should be associated with the multi-user.target when enabled
```

### 4. Enable the Service

Reload the systemd configuration:

```bash
sudo systemctl daemon-reload
```

Enable PocketOwl so that it starts automatically during system startup:

```bash
sudo systemctl enable pocketowl
```

Start the service immediately:

```bash
sudo systemctl start pocketowl
```

Alternatively, both operations can be performed at once:

```bash
sudo systemctl enable --now pocketowl
```

### 5. Check the Service Status

Verify that the service is running:

```bash
sudo systemctl status pocketowl
```

A successful service should show:

```bash
Active: active (running)
```

### 6. View the Application Logs

PocketOwl's standard output and errors are handled by the system journal.

To view the logs:

```bash
sudo journalctl -u pocketowl
```

To follow the logs in real time:

```bash
sudo journalctl -u pocketowl -f
```

To view logs from the current boot only:

```bash
sudo journalctl -u pocketowl -b
```

### 7. Restarting the Service

After updating or recompiling the Rust application, restart the service:

```bash
sudo systemctl restart pocketowl
```

Then verify its status:

```bash
sudo systemctl status pocketowl
```

### 8. Stopping and Disabling the Service

To stop PocketOwl:

```bash
sudo systemctl stop pocketowl
```

To prevent it from starting automatically during boot:

```bash
sudo systemctl disable pocketowl
```

To both stop and disable it:

```bash
sudo systemctl disable --now pocketowl
```

> [!NOTE]
> The Restart=always configuration causes `systemd` to automatically restart the PocketOwl host if the application terminates. This is useful for a monitoring application that is intended to run continuously.