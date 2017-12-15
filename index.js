const { app, BrowserWindow } = require('electron');
const path = require('path');
const url = require('url');

let isDevelopment = true;

if (isDevelopment) {
    require('electron-reload')(__dirname, {
        ignored: /node_modules|[\/\\]\./
    });
}

let win;

function createWindow() {
    win = new BrowserWindow({ width: 800, height: 800 });

    if (isDevelopment) {
        win.webContents.openDevTools();
    }

    win.loadURL(url.format({
        pathname: path.join(__dirname, 'index.html'),
        protocol: 'file:',
        salshes: true
    }))

    //win.webContents.openDevTools();
    win.on('closed', () => {
        win = null;
    })
}

app.on('ready', createWindow);
app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        app.quit();
    }
})

app.on('activate', () => {
    if (win === null) {
        createWindow();
    }
})