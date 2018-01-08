import React, { Component } from 'react';
import { Provider } from 'react-redux';
import configureStore from './redux/store';
import Main from './components/Main';
import './App.css';

const store = configureStore();

export default class App extends Component {
  render() {
    return (
      <Provider store={store}>
        <Main />
      </Provider>
    );
  }
}

