/**
 * Main page show the all connections that include saved in storage.
 * Click one will show the DB instance in the opened tab window.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Modal, Button, Layout, Input, Icon } from 'antd';
import Instance from './Instance';
import { connectDB } from '../redux/thunk/Connect';
import { getAllSavedInstances } from '../redux/thunk/Instance';

const { Content, Sider } = Layout;

class Welcome extends React.PureComponent {

    state = {
        connectionName: '',
        visible: false,
        ip: '10.2.1.128',
        port: 6937
    }

    /**
     * Open connection dialog. Connect the DB after input the connection config.
     * Then create a new tab that contain the DB instance.
     */
    openConnection = () => {
        this.setState({ visible: true })

        //dialog.showOpenDialog({ properties: ['openFile', 'openDirectory', 'multiSelections'] })
    }

    handleConnect = () => {
        const { ip, port, connectionName } = this.state;

        this.props.connectDB({ name: connectionName, ip: ip, port: 6379, password: 'shit' }).then(res => {
            this.props.getAllSavedInstances();
            this.setState({ visible: false });
            this.props.addInstance(ip);
        });
    }

    onCancelConnect = () => {
        this.setState({ visible: false })
    }

    onChangeConnectionName = (e) => {
        this.setState({ connectionName: e.target.value });
    }

    onChangeIP = (e) => {
        this.setState({ ip: e.target.value });
    }

    onChangePort = (e) => {
        this.setState({ port: e.target.value });
    }

    constructSavedInstances = () => {
        const { instances } = this.props;
        if (!instances) {
            return;
        }

        return instances && instances.map((item) => {
            return (
                <li key={item.name} style={{ padding: 5, border: 'solid 1px silver', width: 200, height: 100 }}>
                    <div style={{ fontSize: 18 }}>{item.name || item.ip}</div>
                    <div><Icon type='hdd' />&nbsp;{item.type || 'Redis'}</div>
                    <div><Icon type='link' />&nbsp;{item.ip}:{item.port}</div>
                </li>
            )
        })
    }

    render() {
        const { ip, port, connectionName } = this.state;
        const { instances } = this.props;

        return (
            <Layout>
                <Content style={{ overflow: 'initial', background: "#FFFFFF" }}>
                    <div style={{ margin: '10px 50px', textAlign: 'center' }}>
                        <h1>Welcome to Cube Studio</h1>
                        Cube Studio is a graphical user interface (GUI) tool for Redis & Memcached. It allows you to browse your databases and create, update, delete the data.
                    </div>
                    <div style={{ padding: '10px', background: '#FFFFFF' }}>
                        Connections:&nbsp;<Icon type='plus-circle-o' style={{ color: '#1890FF', fontSize: 18 }} onClick={this.openConnection} />                    </div>
                    <ul style={{ padding: '10px' }}>
                        {this.constructSavedInstances()}
                    </ul>
                </Content>

                <Modal
                    title={'New Connection'}
                    style={{ width: 500 }}
                    visible={this.state.visible}
                    onOk={this.handleConnect}
                    onCancel={this.onCancelConnect}>
                    <div><div style={{ width: 150 }}>Connection Name:</div><Input style={{ width: 300 }} value={connectionName} onChange={this.onChangeIP} /></div>
                    <div><div style={{ width: 150 }}>Hostname:</div><Input style={{ width: 300 }} value={ip} onChange={this.onChangeIP} /></div>
                    <div><div style={{ width: 150 }}>Port:</div><Input style={{ width: 300 }} value={port} onChange={this.onChangePort} /></div>
                </Modal>
            </Layout>
        )
    }

    componentDidMount() {
        this.props.getAllSavedInstances();
    }
}

function mapStateToProps(state) {
    return {
        instances: state.handleInstance.instances || null,
        result: state.handleConnection.result || null
    }
}

export default connect(mapStateToProps, { connectDB, getAllSavedInstances })(Welcome)