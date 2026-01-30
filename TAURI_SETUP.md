# Tauri Setup for Amsterdam Bike Fleet

This document explains how to set up and run the Amsterdam Bike Fleet application as a desktop app using Tauri with a Rust backend.

## Prerequisites

### 1. Install Rust Toolchain

If you don't have Rust installed, install it via rustup:

```bash
# macOS/Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the prompts, then restart your terminal or run:
source $HOME/.cargo/env
```

For Windows, download and run the installer from: https://rustup.rs/

Verify your installation:

```bash
rustc --version
cargo --version
```

### 2. Platform-Specific Dependencies

#### macOS
```bash
xcode-select --install
```

#### Windows
- Install Visual Studio Build Tools with "Desktop development with C++" workload
- Install WebView2 (usually pre-installed on Windows 10/11)

#### Linux (Debian/Ubuntu)
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
```

### 3. Install Node.js Dependencies

```bash
npm install
```

## Project Structure

```
amsterdam-bike-fleet/
├── src/                        # Angular frontend
│   ├── app/
│   │   └── services/
│   │       └── tauri.service.ts  # Angular service for Tauri IPC
│   └── ...
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── main.rs             # Main entry point
│   │   ├── lib.rs              # Library with Tauri setup
│   │   ├── models.rs           # Data models
│   │   ├── database.rs         # SQLite database operations (default)
│   │   ├── database_pg.rs      # PostgreSQL operations (--features postgres)
│   │   └── commands/           # Tauri command handlers
│   │       ├── mod.rs
│   │       ├── fleet.rs        # Fleet commands (SQLite)
│   │       ├── fleet_pg.rs     # Fleet commands (PostgreSQL)
│   │       ├── database.rs     # Database commands (SQLite)
│   │       ├── database_pg.rs  # Database commands (PostgreSQL)
│   │       └── health.rs       # Health check commands
│   ├── Cargo.toml              # Rust dependencies (sqlite/postgres features)
│   ├── tauri.conf.json         # Tauri configuration
│   └── icons/                  # Application icons
└── package.json
```

## Development

### Run Angular Only (Browser)

The Angular app can still run standalone in the browser:

```bash
npm start
# Opens at http://localhost:4200
```

### Run with Tauri (Desktop App)

To run the full desktop application with Rust backend:

```bash
npm run tauri:dev
```

This will:
1. Start the Angular dev server
2. Compile the Rust backend
3. Launch the desktop application window

**Note:** First run will take longer as Rust dependencies are compiled.

### Debug Mode

For development with debug symbols:

```bash
npm run tauri:build:debug
```

## Production Build

### Build for Current Platform

```bash
npm run tauri:build
```

This creates platform-specific installers in `src-tauri/target/release/bundle/`:
- **macOS:** `.dmg` and `.app`
- **Windows:** `.msi` and `.exe`
- **Linux:** `.deb`, `.rpm`, and `.AppImage`

### Cross-Platform Builds

#### Build for macOS (Intel and Apple Silicon)

```bash
# Build for Apple Silicon (M1/M2/M3)
npm run tauri:build
# Output: src-tauri/target/release/bundle/dmg/Amsterdam Bike Fleet_*_aarch64.dmg

# Build for Intel Macs
rustup target add x86_64-apple-darwin
cargo tauri build --target x86_64-apple-darwin
# Output: src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/*.dmg
```

#### Build for Windows

Cross-compiling to Windows from macOS/Linux is complex because Windows bundlers (MSI, NSIS) require Windows-specific tools. **Recommended approaches:**

**Option 1: GitHub Actions (Recommended for CI/CD)**

Push to GitHub and the included workflow (`.github/workflows/build.yml`) will automatically build for Windows:

```bash
git push origin main
# Check Actions tab for Windows build artifacts
```

**Option 2: Build on Windows Machine**

```bash
# On Windows with Visual Studio Build Tools installed
npm run tauri:build
# Output: src-tauri/target/release/bundle/msi/*.msi
# Output: src-tauri/target/release/bundle/nsis/*.exe
```

**Option 3: Docker with Wine (Experimental)**

```bash
# Not officially supported, may have issues with bundlers
docker run --rm -v $(pwd):/app -w /app rustcross/windows:latest cargo tauri build --target x86_64-pc-windows-msvc
```

### GitHub Actions Automated Builds

The project includes a GitHub Actions workflow that builds for all platforms:

- **macOS (Apple Silicon)** - `.dmg` installer
- **macOS (Intel x64)** - `.dmg` installer
- **Windows (x64)** - `.msi` and `.exe` installers

To trigger a release build, create a version tag:

```bash
git tag v1.0.0
git push origin v1.0.0
```

Build artifacts will be available in:
1. GitHub Actions → Workflow runs → Artifacts
2. GitHub Releases (for tagged versions)

## Using the Rust Backend from Angular

### Import the Service

```typescript
import { TauriService } from './services/tauri.service';

@Component({...})
export class MyComponent {
  constructor(private tauri: TauriService) {}

  async loadFleet() {
    if (this.tauri.isTauri()) {
      // Running in desktop app - use Rust backend
      const bikes = await this.tauri.getFleetData();
      console.log('Fleet data from Rust:', bikes);
    } else {
      // Running in browser - use fallback/HTTP API
      console.log('Running in browser mode');
    }
  }
}
```

### Available Commands

| Command | Description | Returns |
|---------|-------------|---------|
| `healthCheck()` | Check Rust backend health | `HealthStatus` |
| `initDatabase()` | Initialize SQLite database | `string` |
| `getDatabaseStats()` | Get database statistics | `DatabaseStats` |
| `getFleetData()` | Get all bikes | `Bike[]` |
| `getBikeById(id)` | Get bike by ID | `Bike \| null` |
| `addBike(request)` | Add new bike | `Bike` |
| `updateBikeStatus(request)` | Update bike status | `void` |
| `getFleetStats()` | Get fleet statistics | `FleetStats` |

### Example: Initialize Database on App Start

```typescript
import { Component, OnInit } from '@angular/core';
import { TauriService } from './services/tauri.service';

@Component({
  selector: 'app-root',
  template: `<router-outlet></router-outlet>`
})
export class AppComponent implements OnInit {
  constructor(private tauri: TauriService) {}

  async ngOnInit() {
    if (this.tauri.isTauri()) {
      try {
        // Initialize database on startup
        const result = await this.tauri.initDatabase();
        console.log(result);

        // Load initial fleet data
        const fleet = await this.tauri.getFleetData();
        console.log(`Loaded ${fleet.length} bikes`);
      } catch (error) {
        console.error('Failed to initialize:', error);
      }
    }
  }
}
```

## Database

The application supports two database backends:

### SQLite (Default)

SQLite is used by default for standalone desktop deployments. The database is stored at:

- **macOS:** `~/Library/Application Support/com.amsterdam-bike-fleet.app/amsterdam_bike_fleet.db`
- **Windows:** `C:\Users\<USER>\AppData\Roaming\com.amsterdam-bike-fleet.app\amsterdam_bike_fleet.db`
- **Linux:** `~/.local/share/com.amsterdam-bike-fleet.app/amsterdam_bike_fleet.db`

On first run, the database is automatically initialized with mock Amsterdam bike data.

### PostgreSQL (On-Premise HA)

For enterprise deployments requiring high availability, the app can be built with PostgreSQL support:

```bash
cd src-tauri
cargo build --release --no-default-features --features postgres
```

Configure via environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `PG_HOST` | PostgreSQL host or HAProxy VIP | localhost |
| `PG_PORT` | PostgreSQL port | 5432 |
| `PG_USER` | Database user | fleet_app |
| `PG_PASSWORD` | Database password | (required) |
| `PG_DATABASE` | Database name | bike_fleet |
| `PG_POOL_SIZE` | Connection pool size | 16 |

See [On-Premise HA Setup](docs/ON_PREMISE_HA_SETUP.md) for complete deployment guide with Patroni, etcd, and HAProxy for 99.99% uptime.

## Application Icons

Place your application icons in `src-tauri/icons/`:

- `32x32.png` - Small icon
- `128x128.png` - Medium icon
- `128x128@2x.png` - Retina medium icon
- `icon.icns` - macOS icon
- `icon.ico` - Windows icon

You can generate these from a single source image using:

```bash
npx tauri icon src-tauri/icons/source.png
```

## Troubleshooting

### Rust Compilation Errors

If you encounter Rust compilation errors:

```bash
# Update Rust
rustup update

# Clean and rebuild
cd src-tauri
cargo clean
cd ..
npm run tauri:build
```

### WebView Issues on Linux

If the app doesn't display properly on Linux:

```bash
# Install WebKit2GTK 4.1
sudo apt install libwebkit2gtk-4.1-dev
```

### Windows Build Issues

Ensure you have:
1. Visual Studio Build Tools with C++ workload
2. WebView2 Runtime (usually pre-installed)

### Database Initialization Fails

Check if the application has write permissions to the app data directory.

## Resources

- [Tauri v2 Documentation](https://v2.tauri.app/)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Angular Documentation](https://angular.io/docs)
