/**
 * Main page show the all connections that include saved in storage.
 * Click one will show the DB instance in the opened tab window.
 * 
 * @author f.achilles
 */

import React from 'react';
import { connect } from 'react-redux';
import { Tabs, Icon } from 'antd';
import { connectDB } from '../redux/thunk/Connect';
import { addInstance, getInstance, getConfig } from '../redux/actions/Instance';
import Welcome from './Welcome';
import Instance from './Instance';

const TabPane = Tabs.TabPane;

class Main extends React.PureComponent {
    constructor(props) {
        super(props);
        this.newTabIndex = 0;
        const panes = [
            { title: <Icon type='home' />, content: <Welcome addInstance={this.add.bind(this)} />, key: '1', closable: false },
            // { title: '10.2.1.128', content: <Instance />, key: '2' }
        ];
        this.state = {
            activeKey: panes[0].key,
            panes,
            visible: false
        };
    }

    onChange = (activeKey) => {
        this.setState({ activeKey });
    }

    onEdit = (targetKey, action) => {
        this[action](targetKey);
    }

    add = (title) => {
        const panes = this.state.panes;
        const activeKey = `newTab${this.newTabIndex++}`;
        panes.push({ title: title, content: <Instance />, key: activeKey });
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

    componentDidUpdate() {
        //this.props.connected && this.add();
    }

    render() {
        const { panes } = this.state;

        return (
            <div>
                <Tabs
                    style={{ marginBottom: 0 }}
                    hideAdd
                    onChange={this.onChange}
                    activeKey={this.state.activeKey}
                    type="editable-card"
                    onEdit={this.onEdit}>
                    {
                        panes.map(pane =>
                            <TabPane tab={pane.title} key={pane.key} closable={pane.closable}>
                                {pane.content}
                            </TabPane>)
                    }
                </Tabs>
            </div>
        );
    }
}



export default connect(state => {
    return {
        instance: state.handleConnection.client
    }
}, { connectDB, addInstance })(Main);