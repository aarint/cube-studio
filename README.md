# Cube Studio

[![Electron](https://img.shields.io/badge/electron-^28.3.0-brightgreen.svg?style=flat-square)](https://github.com/electron/electron)
[![React](https://img.shields.io/badge/react-^18.3.1-brightgreen.svg?style=flat-square)](https://github.com/facebook/react)
[![Ant Design](https://img.shields.io/badge/ant--design-^5.12.8-blue.svg?style=flat-square)](https://github.com/ant-design/ant-design)
[![MIT](https://img.shields.io/dub/l/vibe-d.svg?style=flat-square)](http://opensource.org/licenses/MIT)

Cube Studio is a modern, cross-platform graphical user interface (GUI) tool for Redis & Memcached. Built with Electron and React, it provides an intuitive way to browse databases, manage keys and values, and interact with your NoSQL data stores.

## ✨ Features

### Connection Management
- **Multiple Database Support**: Connect to Redis and Memcached instances
- **Connection Testing**: Test connections before saving
- **Saved Connections**: Securely store connection credentials with encryption
- **Quick Access**: Click saved connections to quickly reconnect

### Data Management
- **Browse Databases**: Navigate through Redis databases (DB0, DB1, etc.) and Memcached keys
- **Key/Value Operations**: 
  - Create new key-value pairs
  - Edit existing values with inline editing
  - Delete keys with confirmation
- **Smart Format Detection**: Automatically detects and formats JSON values
- **Multiple View Modes**: Switch between RAW and JSON format views

### User Experience
- **Modern UI**: Clean, responsive interface built with Ant Design
- **Tab-based Navigation**: Manage multiple connections in separate tabs
- **Real-time Updates**: View changes immediately after operations
- **Error Handling**: Clear error messages for connection and operation failures

## 🚀 Installation

### Prerequisites
- Node.js (v14 or higher)
- npm or yarn

### Build from Source

1. Clone the repository:
```bash
git clone https://github.com/aarint/cube-studio.git
cd cube-studio
```

2. Install dependencies:
```bash
npm install
```

3. Run in development mode:
```bash
npm run dev
```

4. Build for production:
```bash
npm run build
```

5. Package the application:
```bash
npm run package
```

## 📖 Usage

### Connecting to a Database

1. Click the **"New connection"** button on the welcome page
2. Fill in the connection details:
   - **Connection Name**: A friendly name for this connection
   - **Type**: Select Redis or Memcached
   - **Hostname**: Server IP address or hostname
   - **Port**: Server port (default: 6379 for Redis, 11211 for Memcached)
   - **Password**: Authentication password (if required)
   - **Save**: Check to save the connection for future use
3. Click **"Test"** to verify the connection, or **"Connect"** to open the instance

### Managing Data

#### Redis
- Select a database from the dropdown (DB0, DB1, etc.)
- Click on a key in the left panel to view its value
- Use the toolbar buttons to:
  - **Add**: Create a new key-value pair
  - **Edit**: Modify the selected key's value (inline editing)
  - **Delete**: Remove the selected key
- Switch between RAW and JSON view modes using the dropdown

#### Memcached
- View all keys in the left panel
- Click a key to view its value
- Edit or delete keys using the toolbar buttons

### Editing Values

- Click the **Edit** button to enter edit mode
- For JSON values, the editor automatically formats the JSON
- For string values, use the plain text editor
- Click **Save** to apply changes or **Cancel** to discard

## 🛠️ Technology Stack

- **Electron** ^28.3.0 - Cross-platform desktop application framework
- **React** ^18.3.1 - UI library
- **Ant Design** ^5.12.8 - Enterprise UI component library
- **Redux** ^5.0.1 - State management
- **Redux Thunk** ^3.1.0 - Async action handling
- **Redis** ^4.7.0 - Redis client for Node.js
- **node_memcached** ^1.1.3 - Memcached client

## 🔒 Security

- Connection credentials are encrypted before being stored locally
- Passwords are never displayed in plain text
- Uses AES-256-GCM encryption for stored connection data

## 📝 Development

### Project Structure
```
cube-studio/
├── src/
│   ├── components/      # React components
│   │   ├── Welcome/     # Welcome/connection page
│   │   ├── Instance/    # Redis instance view
│   │   └── MemcachedInstance/  # Memcached instance view
│   ├── redux/           # Redux store, actions, reducers, thunks
│   ├── utils/           # Utility functions (encryption, storage, etc.)
│   └── assets/          # Static assets and styles
├── build/               # Webpack configuration
└── main.js             # Electron main process
```

### Available Scripts

- `npm run dev` - Start development server with hot reload
- `npm run build` - Build production bundle
- `npm run package` - Build and package the Electron app
- `npm run prod` - Build and run Electron in production mode

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

Cube Studio is licensed under the [MIT License](LICENSE).

## 👤 Author

**aarint**
- GitHub: [@aarint](https://github.com/aarint)

---

Made with ❤️ using Electron and React
