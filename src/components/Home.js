/**
 * Main page show the all connections that include saved in storage.
 * Click one will show the DB instance in the opened tab window.
 */

import React from 'react';
import { Tabs } from 'antd';
import Main from './Main';
import Instance from './Instance';

const TabPane = Tabs.TabPane;

export default class Home extends React.PureComponent {
    constructor(props) {
        super(props);
        this.newTabIndex = 0;
        const panes = [
            { title: 'Home', content: <Main />, key: '1', closable: false },
            { title: '10.2.1.128', content: <Instance />, key: '2' }
        ];
        this.state = {
            activeKey: panes[0].key,
            panes,
        };
    }

    onChange = (activeKey) => {
        this.setState({ activeKey });
    }

    onEdit = (targetKey, action) => {
        this[action](targetKey);
    }

    add = () => {
        const panes = this.state.panes;
        const activeKey = `newTab${this.newTabIndex++}`;
        panes.push({ title: 'New Tab', content: <Main />, key: activeKey });
        this.setState({ panes, activeKey });
    }

    remove = (targetKey) => {
        let activeKey = this.state.activeKey;
        let lastIndex;
        this.state.panes.forEach((pane, i) => {
            if (pane.key === targetKey) {
                lastIndex = i - 1;
            }
        });
        const panes = this.state.panes.filter(pane => pane.key !== targetKey);
        if (lastIndex >= 0 && activeKey === targetKey) {
            activeKey = panes[lastIndex].key;
        }
        this.setState({ panes, activeKey });
    }

    render() {
        return (
            <Tabs
                onChange={this.onChange}
                activeKey={this.state.activeKey}
                type="editable-card"
                onEdit={this.onEdit}>
                {this.state.panes.map(pane => <TabPane tab={pane.title} key={pane.key} closable={pane.closable}>{pane.content}</TabPane>)}
            </Tabs>
        );
    }
}

const styles = {
    sider: {
        color: 'green',
        overflow: 'auto',
        height: '100vh',
        position: 'fixed',
        left: 0,
        border: 'solid 1 silver',
        background: "black"
    }
}