# Twitch Stream Control for OpenDeck

An open-source Rust plugin for OpenDeck that lets streamers control Twitch chat and stream features directly from their deck hardware. Uses the Twitch Helix API with OAuth2 Device Code Flow for secure authentication.

## Features

Control 11 Twitch actions from your OpenDeck device:

- **Chat Message** — Send preset messages to chat
- **Clear Chat** — Instantly clear the chat
- **Emote Only Chat** — Toggle emote-only mode
- **Followers Only Chat** — Toggle followers-only mode (configurable minimum follow time)
- **Subscribers Only Chat** — Toggle subscribers-only mode
- **Slow Mode Chat** — Toggle slow mode (configurable delay in seconds)
- **Play Ad** — Run an advertisement (30-180s, Partner/Affiliate status required)
- **Stream Marker** — Create a marker at the current stream timestamp
- **Create Clip** — Generate a clip from the current broadcast
- **Viewer Count** — Display live viewer count (updates every 30 seconds)
- **Shield Mode** — Toggle Twitch Shield Mode for spam/harassment protection

## Setup

### 1. Create a Twitch Application

1. Go to [Twitch Developer Console](https://dev.twitch.tv/console/apps)
2. Create a new application
   - **Application Name**: OpenDeck Twitch Plugin
   - **Application Category**: Choose appropriate category (e.g., "Other")
   - **Redirect URI**: `http://localhost`
3. Accept the Developer Agreement and click **Create**
4. Copy your **Client ID** (you'll need this in step 4)

### 2. Install OpenDeck

Follow the [OpenDeck installation guide](https://opendeck.io/wiki/software/) for your operating system.

### 3. Install the Plugin

```bash
# Clone or download this repository
git clone https://github.com/penguintechinc/twitch-opendeck.git
cd twitch-opendeck

# Install the plugin
make install
```

This copies the plugin to:
- **macOS/Linux**: `~/.config/opendeck/plugins/dev.penguin.twitch.sdPlugin/`
- **Windows**: `%APPDATA%\Elgato\StreamDeck\Plugins\dev.penguin.twitch.sdPlugin\`

### 4. Add a Twitch Action to Your Deck

1. Open OpenDeck
2. Add any Twitch action to a button (Chat Message, Viewer Count, etc.)
3. The Property Inspector appears on the right
4. Enter your **Client ID** in the Client ID field
5. Click **Login with Twitch**

### 5. Authenticate with Twitch

1. A popup window displays a device code and authorization URL
2. Visit the URL in your browser
3. Enter the device code when prompted
4. Authorize the plugin to access your Twitch account
5. Return to OpenDeck — authentication is complete

### Done

Your OpenDeck device is now authenticated with Twitch. Actions will execute immediately when pressed.

## Building from Source

### Prerequisites

- Rust 1.70 or later
- OpenDeck (installed)

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Build and Install

```bash
git clone https://github.com/penguintechinc/twitch-opendeck.git
cd twitch-opendeck

# Build and install the plugin
make install

# Or build only
make build

# Or develop with live reload
make dev
```

## Important Notes

### Permissions & Restrictions

- **Moderation Actions** (Clear Chat, Emote Only, Followers Only, Subscribers Only, Slow Mode, Shield Mode) require you to be:
  - The channel broadcaster, OR
  - A moderator in the channel
  
- **Play Ad** requires:
  - Partner or Affiliate status on your Twitch channel

- **Create Clip** requires:
  - The channel to be live at the time you press the button

- **Viewer Count** works with any authentication but displays 0 if the channel is offline

### Token Management

Tokens are automatically:
- Persisted in OpenDeck's global settings (encrypted)
- Refreshed before expiration
- Validated on each button press

You can revoke access at any time from your [Twitch Security Settings](https://www.twitch.tv/settings/connections).

## Configuration Per Action

Some actions support customization via the Property Inspector:

| Action | Configurable Fields |
|--------|-------------------|
| Chat Message | Preset message text |
| Followers Only Chat | Minimum follow time (minutes) |
| Slow Mode Chat | Delay duration (seconds, 0-120) |
| Play Ad | Duration (30, 60, 90, 120, 180 seconds) |

## Troubleshooting

**"Plugin not found"** — Run `make install` again to ensure the plugin directory structure is correct.

**"Login failed"** — Verify your Client ID is correct. Check that the Redirect URI in your Twitch app is exactly `http://localhost`.

**"Action failed: Not authorized"** — Ensure you are a moderator in the channel or the channel broadcaster.

**"Viewer count shows 0"** — The channel may be offline. Viewer count only displays for live streams.

## Development

### Project Structure

```
twitch-opendeck/
├── src/
│   ├── main.rs              # Plugin entry point and message dispatcher
│   ├── auth.rs              # OAuth2 Device Code Flow implementation
│   ├── settings.rs          # OpenDeck property persistence
│   ├── twitch_api.rs        # Helix API client
│   ├── global_handler.rs    # Global plugin state and initialization
│   └── actions/             # Individual action implementations
│       ├── chat_message.rs
│       ├── clear_chat.rs
│       ├── emote_chat.rs
│       └── ... (other actions)
├── Cargo.toml               # Rust dependencies
├── Makefile                 # Build and install targets
└── README.md                # This file
```

### Running Tests

```bash
make test
```

## License

This plugin is open source. See LICENSE file for details.

## Support

For issues, feature requests, or contributions, visit the [GitHub Issues](https://github.com/penguintechinc/twitch-opendeck/issues) page.
