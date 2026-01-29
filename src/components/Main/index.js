/**
 * Main page show all connections that include saved in storage.
 * Click one will show DB instance in the opened tab window.
 *
 * @author f.achilles
 */

import React from 'react';
import { connect } from 'react-redux';
import { Tabs } from 'antd';
import { HomeOutlined, SmileOutlined } from '@ant-design/icons';
import { connectDB } from '../../redux/thunk/Connect';
import { addInstance } from '../../redux/actions/Instance';
import { getAllSavedInstances } from '../../redux/thunk/Instance';
import Welcome from '../Welcome';
import Instance from '../Instance';
import MemcachedInstance from '../MemcachedInstance';

import '../../assets/styles/index.css';

class Main extends React.PureComponent {
    constructor(props) {
        super(props);
        this.newTabIndex = 0;
        const panes = [
            { title: <HomeOutlined />, content: <Welcome addInstance={this.add.bind(this)} />, key: '1', closable: false },
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

    add = (title, type = 'Redis') => {
        const panes = this.state.panes;
        const activeKey = `newTab${this.newTabIndex++}`;
        const content = type === 'Memcached' ? <MemcachedInstance /> : <Instance />;
        panes.push({ title: title, content, key: activeKey });
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
        const { panes } = this.state;

        const items = panes.map(pane => ({
            key: pane.key,
            label: pane.title,
            children: pane.content,
            closable: pane.closable,
        }));

        return (
            <div>
                <Tabs
                    className="main-tabs"
                    style={{ height: 'calc(100vh - 22px)' }}
                    hideAdd
                    onChange={this.onChange}
                    activeKey={this.state.activeKey}
                    type="editable-card"
                    onEdit={this.onEdit}
                    items={items}
                />
                <footer className="toolbar toolbar-footer">
                    <div className="title"><span>UTF-8</span>&nbsp;<SmileOutlined /></div>
                </footer>
            </div>
        );
    }
}

export default connect(state => {
    return {
        instance: state.handleConnection.client
    }
}, { connectDB, addInstance, getAllSavedInstances })(Main);
