const webpack = require("webpack");
const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const { spawn } = require("child_process");
const ReactRefreshWebpackPlugin = require('@pmmmwh/react-refresh-webpack-plugin');

// Config directories
const SRC_DIR = path.resolve(__dirname, "../src");
const OUTPUT_DIR = path.resolve(__dirname, "../dist");

// Any directories you will be adding code/files into, need to be added to this array so webpack will pick them up
const defaultInclude = [SRC_DIR];

module.exports = {
  mode: "development",
  entry: SRC_DIR + "/index.js",
  output: {
    path: OUTPUT_DIR,
    publicPath: "/",
    filename: "bundle.js",
    clean: true,
  },
  externals: {
    electron: 'require("electron")',
    redis: 'require("redis")',
    net: 'require("net")',
    remote: 'require("remote")',
    shell: 'require("shell")',
    app: 'require("app")',
    ipc: 'require("ipc")',
    fs: 'require("fs")',
    buffer: 'require("buffer")',
    system: "{}",
    file: "{}",
  },
  module: {
    rules: [
      {
        test: /\.module\.css$/,
        use: [
          { loader: "style-loader" },
          {
            loader: "css-loader",
            options: {
              modules: {
                localIdentName: "[name]__[local]__[hash:base64:5]",
              },
            },
          },
        ],
        include: defaultInclude,
      },
      {
        test: /\.css$/,
        exclude: /\.module\.css$/,
        use: [{ loader: "style-loader" }, { loader: "css-loader" }],
        include: defaultInclude,
      },
      {
        test: /\.jsx?$/,
        use: [
          {
            loader: "babel-loader",
            options: {
              plugins: [require.resolve('react-refresh/babel')],
            },
          },
        ],
        include: defaultInclude,
      },
      {
        test: /\.(jpe?g|png|gif)$/,
        type: "asset",
        generator: {
          filename: "img/[name]__[hash:base64:5][ext]",
        },
        include: defaultInclude,
      },
      {
        test: /\.(eot|svg|ttf|woff|woff2)$/,
        type: "asset/resource",
        generator: {
          filename: "font/[name]__[hash:base64:5][ext]",
        },
        include: defaultInclude,
      },
    ],
  },
  target: "electron-renderer",
  resolve: {
    alias: {
      "@": SRC_DIR,
    },
    extensions: ['.js', '.jsx', '.json'],
  },
  plugins: [
    new HtmlWebpackPlugin(),
    new ReactRefreshWebpackPlugin(),
    new webpack.DefinePlugin({
      "process.env.NODE_ENV": JSON.stringify("development"),
    }),
  ],
  devtool: "cheap-source-map",
  devServer: {
    static: {
      directory: OUTPUT_DIR,
    },
    hot: true,
    open: false,
    devMiddleware: {
      writeToDisk: true,
    },
    client: {
      overlay: false,
    },
    setupMiddlewares: (middlewares, devServer) => {
      spawn("electron", ["."], {
        shell: true,
        env: process.env,
        stdio: "inherit",
      })
        .on("close", (code) => process.exit(0))
        .on("error", (spawnError) => console.error(spawnError));

      return middlewares;
    },
  },
};
