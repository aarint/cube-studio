import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';

// Since we are using HtmlWebpackPlugin WITHOUT a template, we should create our own root node in the body element before rendering into it
let root = document.createElement('div');
root.id = "root";
document.body.appendChild(root);

// Get or create the root container
const rootContainer = document.getElementById('root');

// Initialize React 18 root
let reactRoot = createRoot(rootContainer);

// Render the app
reactRoot.render(<App />);

// HMR (Hot Module Replacement) support for development
if (module.hot) {
    module.hot.accept('./App', () => {
        const NextApp = require('./App').default;
        reactRoot.render(<NextApp />);
    });
}
