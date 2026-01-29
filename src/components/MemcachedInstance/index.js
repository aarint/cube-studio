/**
 * The Memcached instance will be shown in a new tab.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Button, Layout, notification, List, Input, Modal } from 'antd';
import { DeleteOutlined, EditOutlined } from '@ant-design/icons';
import { getMemcachedKeysList, getMemcachedObjectByKey, setMemcachedValue, deleteMemcachedValue } from '../../redux/thunk/Memcached';

const { Content, Sider } = Layout;

class MemcachedInstance extends React.PureComponent {
    state = {
        currentKey: null,
        currentValue: null,
        visible: false,
        valueInput: ''
    }

    constructor(props) {
        super(props);

        this.curInstance = this.props.curInstance;
    }

    constructKeys() {
        const { keys } = this.props;

        if (!keys) {
            return null;
        }

        return (
            <List
                dataSource={keys}
                renderItem={(key, index) => (
                    <List.Item
                        key={index}
                        onClick={() => this.onSelectKey(key)}
                        className={this.state.currentKey === key ? 'active-key' : ''}
                        style={{ cursor: 'pointer', padding: '12px 16px' }}
                    >
                        <span>{key}</span>
                    </List.Item>
                )}
                style={{ border: '1px solid #f0f0f0', borderRadius: 4, height: 'calc(100vh - 200px)', overflowY: 'auto' }}
            />
        );
    }

    onSelectKey = (key) => {
        this.setState({ currentKey: key });
        this.props.getMemcachedObjectByKey(key);
    }

    onEdit = () => {
        const { obj } = this.props;
        if (obj) {
            this.setState({ visible: true, valueInput: obj.value });
        }
    }

    onCancel = () => {
        this.setState({ visible: false });
    }

    onOk = () => {
        const { currentKey, valueInput } = this.state;
        if (currentKey) {
            this.props.setMemcachedValue(currentKey, valueInput);
        }
        this.setState({ visible: false });
    }

    onDelete = () => {
        const { currentKey } = this.state;
        if (currentKey) {
            this.props.deleteMemcachedValue(currentKey);
        }
    }

    onChangeValue = (e) => {
        this.setState({ valueInput: e.target.value });
    }

    render() {
        const { currentKey, currentValue, visible, valueInput } = this.state;
        const { instance, keys, obj } = this.props;

        return (
            <div className="instance-container">
                <div className="instance-sidebar">
                    <div className="instance-sidebar-header">
                        <div style={{ height: 30, paddingLeft: 10 }}>Memcached Keys</div>
                    </div>
                    <div className="instance-keys-list">
                        {this.constructKeys()}
                    </div>
                </div>
                <div className="instance-content">
                    <div className="instance-top-bar">
                        <div className="instance-actions">
                            {obj && <span style={{ color: 'green' }}>{obj.type}</span>}
                            <DeleteOutlined onClick={() => this.onDelete()} style={{ marginLeft: 8, cursor: 'pointer' }} />
                            <EditOutlined onClick={() => this.onEdit()} style={{ marginLeft: 8, cursor: 'pointer' }} />
                        </div>
                    </div>
                    <div className="instance-value-display">
                        <div className="instance-value-content">
                            {currentKey && obj ? (
                                <div>
                                    <div style={{ marginBottom: 16 }}>
                                        <strong>Key:</strong> {obj.key}
                                    </div>
                                    <div style={{ marginBottom: 16 }}>
                                        <strong>Value:</strong>
                                    </div>
                                    <div style={{ padding: 16, background: '#f5f5f5', borderRadius: 4, fontFamily: 'Courier New, Courier, monospace', fontSize: 13 }}>
                                        {obj.value || '(empty)'}
                                    </div>
                                </div>
                            ) : (
                                <div style={{ padding: 16, color: '#999' }}>
                                    Select a key to view its value
                                </div>
                            )}
                        </div>
                    </div>
                </div>

                <Modal
                    title="Edit Value"
                    open={visible}
                    onOk={this.onOk}
                    onCancel={this.onCancel}
                >
                    <Input.TextArea
                        value={valueInput}
                        onChange={this.onChangeValue}
                        autoSize={{ minRows: 3, maxRows: 10 }}
                        placeholder="Enter value"
                    />
                </Modal>
            </div>
        );
    }

    componentDidMount() {
        this.props.getMemcachedKeysList();
    }
}

function mapStateToProps(state) {
    return {
        obj: state.handleMemcached.obj || null,
        keys: state.handleMemcached.keys || null,
        instance: state.handleConnection.client || null
    }
}

export default connect(mapStateToProps, {
    getMemcachedKeysList,
    getMemcachedObjectByKey,
    setMemcachedValue,
    deleteMemcachedValue
})(MemcachedInstance);
