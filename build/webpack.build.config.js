const webpack = require('webpack');
const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');

// Config directories
const SRC_DIR = path.resolve(__dirname, 'src');
const OUTPUT_DIR = path.resolve(__dirname, 'dist');

// Any directories you will be adding code/files into, need to be added to this array so webpack will pick them up
const defaultInclude = [SRC_DIR];

module.exports = {
  mode: 'production',
  entry: SRC_DIR + '/index.js',
  output: {
    path: OUTPUT_DIR,
    publicPath: './',
    filename: 'bundle.js',
    clean: true
  },
  module: {
    rules: [
      {
        test: /\.module\.css$/,
        use: [
          MiniCssExtractPlugin.loader,
          {
            loader: 'css-loader',
            options: {
              modules: {
                localIdentName: '[name]__[local]__[hash:base64:5]',
              },
            },
          },
        ],
        include: defaultInclude
      },
      {
        test: /\.css$/,
        exclude: /\.module\.css$/,
        use: [MiniCssExtractPlugin.loader, 'css-loader'],
        include: defaultInclude
      },
      {
        test: /\.jsx?$/,
        use: [{ loader: 'babel-loader' }],
        include: defaultInclude
      },
      {
        test: /\.(jpe?g|png|gif)$/,
        type: "asset",
        generator: {
          filename: "img/[name]__[hash:base64:5][ext]",
        },
        include: defaultInclude
      },
      {
        test: /\.(eot|svg|ttf|woff|woff2)$/,
        type: "asset/resource",
        generator: {
          filename: "font/[name]__[hash:base64:5][ext]",
        },
        include: defaultInclude
      }
    ]
  },
  target: 'electron-renderer',
  resolve: {
    alias: {
        '@': SRC_DIR
    },
    extensions: ['.js', '.jsx', '.json'],
  },
  optimization: {
    minimize: true,
  },
  plugins: [
    new HtmlWebpackPlugin(),
    new MiniCssExtractPlugin({
      filename: 'bundle.css',
    }),
    new webpack.DefinePlugin({
      'process.env.NODE_ENV': JSON.stringify('production')
    })
  ],
  stats: {
    colors: true,
    children: false,
    chunks: false,
    modules: false
  }
};
