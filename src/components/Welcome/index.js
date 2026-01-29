/**
 * Main page show the all connections that include saved in storage.
 * Click one will show the DB instance in the opened tab window.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Modal, Button, Layout, Input, Select, notification, Form, Checkbox } from 'antd';
import { PlusCircleOutlined, HddOutlined, LinkOutlined } from '@ant-design/icons';
import { connectDB, testConnectDB } from '../../redux/thunk/Connect';
import { connectMemcached, testConnectMemcached } from '../../redux/thunk/Memcached';
import { addConnectedInstance, getAllSavedInstances } from '../../redux/thunk/Instance';
import { upsertSavedInstance } from '../../utils/InstanceUtil';

const { Content, Sider } = Layout;

class Welcome extends React.PureComponent {

    state = {
        alias: '',
        visible: false,
        type: 'Redis',
        ip: '127.0.0.1',
        port: 6379,
        password: '',
        isSmall: false,
        save: true
    }

    updateIsSmall = () => {
        // Antd md breakpoint is 768px by default
        const isSmall = typeof window !== 'undefined' ? window.innerWidth < 768 : false;
        if (isSmall !== this.state.isSmall) {
            this.setState({ isSmall });
        }
    }

    /**
     * Open connection dialog. Connect the DB after input the connection config.
     * Then create a new tab that contain the DB instance.
     */
    openConnection = () => {
        this.setState({ visible: true })
        this.updateIsSmall();

        //dialog.showOpenDialog({ properties: ['openFile', 'openDirectory', 'multiSelections'] })
    }

    handleConnect = () => {
        const { ip, port, alias, password, type, save } = this.state;

        const config = {
            name: alias,
            ip,
            port: Number(port),
            password,
            type,
            save
        };

        const connectAction = type === 'Memcache' ? this.props.connectMemcached : this.props.connectDB;

        return connectAction(config)
            .then(() => {
                if (save) {
                    upsertSavedInstance(config);
                }
                this.props.getAllSavedInstances();
                this.setState({ visible: false });
            })
            .catch((err) => {
                const message =
                    (err && (err.message || err.details || err.toString && err.toString())) ||
                    'Unknown connection error';
                notification.error({
                    message: 'Connection failed',
                    description: message
                });
            });
    }

    handleTest = () => {
        const { ip, port, password, type, alias, save } = this.state;

        const config = {
            name: alias,
            ip,
            port: Number(port),
            password,
            type,
            save
        };

        const testAction = type === 'Memcache' ? this.props.testConnectMemcached : this.props.testConnectDB;

        return testAction(config)
            .then(() => {
                if (save) {
                    upsertSavedInstance(config);
                    this.props.getAllSavedInstances();
                }
                notification.success({
                    message: 'Connection OK',
                    description: `${type} ${ip}:${Number(port)}`
                });
            })
            .catch((err) => {
                const message =
                    (err && (err.message || err.details || (err.toString && err.toString()))) ||
                    'Unknown connection error';
                notification.error({
                    message: 'Connection failed',
                    description: message
                });
            });
    }

    onCancelConnect = () => {
        this.setState({ visible: false })
    }

    onChangeConnectionName = (e) => {
        this.setState({ alias: e.target.value });
    }

    onChangeIP = (e) => {
        this.setState({ ip: e.target.value, alias: e.target.value });
    }

    onChangePort = (e) => {
        this.setState({ port: e.target.value });
    }

    onChangeType = (value) => {
        const nextPort = value === 'Memcache' ? 11211 : 6379;
        this.setState({ type: value, port: nextPort });
    }

    onChangePassword = (e) => {
        this.setState({ password: e.target.value });
    }

    onChangeSave = (e) => {
        this.setState({ save: e.target.checked });
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
                    <div><HddOutlined />&nbsp;{item.type || 'Redis'}</div>
                    <div><LinkOutlined />&nbsp;{item.ip}:{item.port}</div>
                </li>
            )
        })
    }

    render() {
        const { ip, port, alias, type, password, save } = this.state;
        const { instances } = this.props;
        const { isSmall } = this.state;

        return (
            <Layout>
                <Content style={{ overflow: 'initial', background: "#FFFFFF" }}>
                    <div style={{ margin: '10px 50px', textAlign: 'center' }}>
                        <h1>Welcome to Cube Studio</h1>
                        Cube Studio is a graphical user interface (GUI) tool for Redis & Memcached. It allows you to browse your databases and create, update, delete the data.
                    </div>
                    <div style={{ padding: '10px', background: '#FFFFFF' }}>
                        Connections:&nbsp;<PlusCircleOutlined style={{ color: '#1890FF', fontSize: 18 }} onClick={this.openConnection} />                    </div>
                    <ul style={{ padding: '10px' }}>
                        {this.constructSavedInstances()}
                    </ul>
                </Content>

                <Modal
                    title={'New Connection'}
                    width={isSmall ? '92vw' : 520}
                    visible={this.state.visible}
                    onOk={this.handleConnect}
                    onCancel={this.onCancelConnect}
                    footer={[
                        <Button key="cancel" onClick={this.onCancelConnect}>Cancel</Button>,
                        <Button key="test" onClick={this.handleTest}>Test</Button>,
                        <Button key="ok" type="primary" onClick={this.handleConnect}>Connect</Button>,
                    ]}>
                    <Form
                        layout="vertical"
                        colon={false}
                    >
                        <Form.Item label="Connection Name">
                            <Input value={alias} onChange={this.onChangeConnectionName} />
                        </Form.Item>
                        <Form.Item label="Type">
                            <Select
                                value={type}
                                onChange={this.onChangeType}
                                options={[
                                    { value: 'Redis', label: 'Redis' },
                                    { value: 'Memcache', label: 'Memcache' }
                                ]}
                            />
                        </Form.Item>
                        <Form.Item label="Hostname">
                            <Input value={ip} onChange={this.onChangeIP} />
                        </Form.Item>
                        <Form.Item label="Port">
                            <Input value={port} onChange={this.onChangePort} />
                        </Form.Item>
                        <Form.Item label="Password">
                            <Input.Password value={password} onChange={this.onChangePassword} />
                        </Form.Item>
                        <Form.Item>
                            <Checkbox checked={save} onChange={this.onChangeSave}>
                                Save
                            </Checkbox>
                        </Form.Item>
                    </Form>
                </Modal>
            </Layout>
        )
    }

    componentDidMount() {
        this.props.getAllSavedInstances();
        this.updateIsSmall();
        if (typeof window !== 'undefined') {
            window.addEventListener('resize', this.updateIsSmall);
        }
    }

    componentWillUnmount() {
        if (typeof window !== 'undefined') {
            window.removeEventListener('resize', this.updateIsSmall);
        }
    }
}

function mapStateToProps(state) {
    return {
        instances: state.handleInstance.instances || null,
        result: state.handleConnection.result || null
    }
}

export default connect(mapStateToProps, {
    connectDB,
    testConnectDB,
    connectMemcached,
    testConnectMemcached,
    getAllSavedInstances
})(Welcome)