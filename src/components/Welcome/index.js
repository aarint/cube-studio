/**
 * Main page show the all connections that include saved in storage.
 * Click one will show the DB instance in the opened tab window.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Modal, Button, Layout, Input, Select, notification, Form, Checkbox, Card, Row, Col, Typography, Empty } from 'antd';
import { PlusCircleOutlined, HddOutlined, LinkOutlined } from '@ant-design/icons';
import { connectDB, testConnectDB } from '../../redux/thunk/Connect';
import { connectMemcached, testConnectMemcached } from '../../redux/thunk/Memcached';
import { addConnectedInstance, getAllSavedInstances } from '../../redux/thunk/Instance';
import { upsertSavedInstance } from '../../utils/InstanceUtil';

const { Content } = Layout;
const { Title, Paragraph } = Typography;

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

        const connectAction = type === 'Memcached' ? this.props.connectMemcached : this.props.connectDB;

        return connectAction(config)
            .then(() => {
                if (save) {
                    upsertSavedInstance(config);
                }
                this.props.getAllSavedInstances();
                this.setState({ visible: false });
                if (typeof this.props.addInstance === 'function') {
                    const title = config.name || `${config.ip}:${config.port}`;
                    this.props.addInstance(title, config.type);
                }
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

        const testAction = type === 'Memcached' ? this.props.testConnectMemcached : this.props.testConnectDB;

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
        const nextPort = value === 'Memcached' ? 11211 : 6379;
        this.setState({ type: value, port: nextPort });
    }

    onChangePassword = (e) => {
        this.setState({ password: e.target.value });
    }

    onChangeSave = (e) => {
        this.setState({ save: e.target.checked });
    }

    renderSavedInstances = () => {
        const { instances } = this.props;
        if (!instances || instances.length === 0) {
            return (
                <Empty
                    className="welcome-empty"
                    description="No saved connections"
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                >
                    <Button type="primary" icon={<PlusCircleOutlined />} onClick={this.openConnection}>
                        Add connection
                    </Button>
                </Empty>
            );
        }

        return (
            <Row gutter={[16, 16]}>
                {instances.map((item) => (
                    <Col key={`${item.type || 'Redis'}-${item.name || item.ip}-${item.port}`} xs={24} sm={12} md={8} lg={6}>
                        <Card
                            className="welcome-connection-card"
                            hoverable
                            onClick={() => this.handleOpenSaved(item)}
                        >
                            <div className="welcome-card-title">{item.name || item.ip}</div>
                            <div className="welcome-card-meta">
                                <span><HddOutlined /> {item.type || 'Redis'}</span>
                                <span><LinkOutlined /> {item.ip}:{item.port}</span>
                            </div>
                        </Card>
                    </Col>
                ))}
            </Row>
        );
    }

    handleOpenSaved = (item) => {
        if (!item) return;
        const type = item.type === 'Memcache' ? 'Memcached' : (item.type || 'Redis');
        const config = {
            name: item.name,
            ip: item.ip,
            port: Number(item.port),
            password: item.password || '',
            type,
            save: true,
        };

        const connectAction = type === 'Memcached' ? this.props.connectMemcached : this.props.connectDB;
        return connectAction(config)
            .then(() => {
                if (typeof this.props.addInstance === 'function') {
                    const title = config.name || `${config.ip}:${config.port}`;
                    this.props.addInstance(title, type);
                }
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

    render() {
        const { ip, port, alias, type, password, save, isSmall } = this.state;
        const { instances } = this.props;

        return (
            <Layout className="welcome-layout">
                <Content className="welcome-content">
                    <div className="welcome-hero">
                        <Title level={2} className="welcome-title">Welcome to Cube Studio</Title>
                        <Paragraph className="welcome-desc">
                            A graphical interface for Redis & Memcached. Browse databases and manage keys and values.
                        </Paragraph>
                    </div>
                    <div className="welcome-section">
                        <div className="welcome-section-header">
                            <Title level={5} style={{ margin: 0 }}>Connections</Title>
                            <Button type="primary" icon={<PlusCircleOutlined />} onClick={this.openConnection}>
                                New connection
                            </Button>
                        </div>
                        <div className="welcome-connections">
                            {this.renderSavedInstances()}
                        </div>
                    </div>
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
                        layout={isSmall ? "vertical" : "horizontal"}
                        labelCol={isSmall ? undefined : { span: 7 }}
                        wrapperCol={isSmall ? undefined : { span: 17 }}
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
                                    { value: 'Memcached', label: 'Memcache' }
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