import React, { Component } from "react";
import { Provider } from "react-redux";
import { ConfigProvider } from "antd";
import configureStore from "@/redux/store";
import { Main } from "@/components";
import "./App.css";

const store = configureStore();

// Custom theme configuration
const customTheme = {
    token: {
        colorPrimary: '#1890ff',
        borderRadius: 4,
        fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
        fontSize: 14,
    },
    components: {
        Layout: {
            bodyBg: '#f0f2f5',
            headerBg: '#001529',
            siderBg: '#ffffff',
        },
        Card: {
            colorBorder: '#d9d9d9',
            boxShadow: '0 1px 2px 0 rgba(0, 0, 0, 0.03)',
        },
        List: {
            itemBg: '#ffffff',
            itemHoverBg: '#f5f5f5',
        },
        Modal: {
            contentBg: '#ffffff',
        },
        Input: {
            colorBg: '#ffffff',
            colorBorder: '#d9d9d9',
        },
        Tag: {
            borderRadius: 4,
        },
    },
};

export default class App extends Component {
    render() {
        return (
            <ConfigProvider theme={customTheme}>
                <Provider store={store}>
                    <Main />
                </Provider>
            </ConfigProvider>
        );
    }
}
