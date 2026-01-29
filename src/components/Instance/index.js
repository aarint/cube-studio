/**
 * The DB instance will be show in new tab.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Button, Layout, Tree, notification, List, Select, Modal, Input, Popconfirm } from 'antd';
import { DeleteOutlined, EditOutlined } from '@ant-design/icons';
import { getCurrentInstanceKeys, getConfigByKey, setConfigByKey, getObjectByKey, changeDataBase, setKeyValue, deleteKey } from '../../redux/thunk/Redis';

require('json-editor');

const { Content, Sider } = Layout;

function isJSON(str) {
    if (typeof str === "string") {
        try {
            JSON.parse(str);
            return true;
        } catch (ex) {
            return false;
        }
    }
}

class Instance extends React.PureComponent {
    state = {
        currentKey: null,
        currentValue: null,
        viewer: 'RAW',
        selectedDB: 'DB0',
        kvModalOpen: false,
        kvModalMode: 'add', // 'add' | 'edit'
        kvKeyInput: '',
        kvValueInput: ''
    }

    constructor(props) {
        super(props);

        this.curInstance = this.props.curInstance;
    }

    constructDBTree() {
        const { config } = this.props;

        if (!config) {
            return [];
        }

        let treeNodes = [];
        for (let i = 1; i <= parseInt(config.value); i++) {
            treeNodes.push({ title: `DB${i}`, key: `${i}` });
        }

        return treeNodes;
    }

    constructOptions() {
        const { config, obj } = this.props;

        if (!config) {
            return [];
        }

        let options = [];
        for (let i = 0; i < parseInt(config.value); i++) {
            options.push({ key: `${i}`, value: `DB${i}`, label: `DB${i}` })
        }

        return options;
    }

    constructKeys() {
        const { keys, isChanging } = this.props;

        if (!keys || isChanging) {
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
                style={{ border: 'none' }}
            />
        );
    }

    onSelectKey = (key) => {
        this.setState({ currentKey: key });
        this.props.getObjectByKey(key);
    }

    onSelectNode = (keys, info) => {
        console.log(keys, info);
    }

    onChangeViewer = (value) => {
        this.setState({ viewer: value })
    }

    onSelectDB = (value, option) => {
        this.setState({ selectedDB: value, currentKey: null });
        const db = value.substring(2);
        this.props.changeDataBase(db);
    }

    openAddKeyValue = () => {
        this.setState({
            kvModalOpen: true,
            kvModalMode: 'add',
            kvKeyInput: '',
            kvValueInput: ''
        });
    }

    openEditKeyValue = () => {
        const { obj } = this.props;
        if (!obj || !obj.key) return;
        this.setState({
            kvModalOpen: true,
            kvModalMode: 'edit',
            kvKeyInput: obj.key,
            kvValueInput: obj.value || ''
        });
    }

    closeKeyValueModal = () => {
        this.setState({ kvModalOpen: false });
    }

    onChangeKVKey = (e) => {
        this.setState({ kvKeyInput: e.target.value });
    }

    onChangeKVValue = (e) => {
        this.setState({ kvValueInput: e.target.value });
    }

    submitKeyValue = () => {
        const { kvKeyInput, kvValueInput, kvModalMode } = this.state;
        const key = (kvKeyInput || '').trim();
        if (!key) {
            notification.warning({ message: 'Key is required' });
            return;
        }

        return this.props.setKeyValue(key, kvValueInput || '')
            .then(() => {
                this.setState({ kvModalOpen: false, currentKey: key });
                this.props.getObjectByKey(key);
                notification.success({ message: kvModalMode === 'edit' ? 'Updated' : 'Added' });
            })
            .catch((err) => {
                notification.error({
                    message: 'Operation failed',
                    description: (err && (err.message || (err.toString && err.toString()))) || 'Unknown error'
                });
            });
    }

    onDeleteKey = () => {
        const { currentKey } = this.state;
        if (!currentKey) return;
        return this.props.deleteKey(currentKey)
            .then(() => {
                notification.success({ message: 'Deleted' });
                this.setState({ currentKey: null });
            })
            .catch((err) => {
                notification.error({
                    message: 'Delete failed',
                    description: (err && (err.message || (err.toString && err.toString()))) || 'Unknown error'
                });
            });
    }

    constructJSONOption = (obj) => {
        const options = [{ key: 'RAW', value: 'RAW', label: 'RAW' }];

        if (obj && obj.value && isJSON(obj.value)) {
            options.push({ key: 'JSON', value: 'JSON', label: 'JSON' });
        }

        return options;
    }

    render() {
        const { currentKey, viewer, selectedDB, kvModalOpen, kvModalMode, kvKeyInput, kvValueInput } = this.state;
        const { instance, keys, config, obj } = this.props;

        return (
            <div className="instance-container">
                <div className="instance-sidebar">
                    <div className="instance-sidebar-header">
                        <div className="instance-sidebar-title">Redis DB</div>
                        <Select
                            style={{ width: '100%' }}
                            value={selectedDB}
                            onSelect={this.onSelectDB}
                            options={this.constructOptions()}
                        />
                        <Button
                            style={{ marginTop: 10, width: '100%' }}
                            type="primary"
                            onClick={this.openAddKeyValue}
                        >
                            Add Key/Value
                        </Button>
                    </div>
                    <div className="instance-keys-list">
                        {this.constructKeys()}
                    </div>
                </div>
                <div className="instance-content">
                    <div className="instance-top-bar">
                        <div className="instance-actions">
                            {obj && <span style={{ fontSize: 14 }}>{obj.type}</span>}
                            <Popconfirm
                                title="Delete this key?"
                                okText="Delete"
                                cancelText="Cancel"
                                onConfirm={this.onDeleteKey}
                                disabled={!currentKey}
                            >
                                <DeleteOutlined style={{ cursor: currentKey ? 'pointer' : 'not-allowed', color: currentKey ? undefined : '#bbb' }} />
                            </Popconfirm>
                            <EditOutlined onClick={this.openEditKeyValue} style={{ cursor: currentKey ? 'pointer' : 'not-allowed', color: currentKey ? undefined : '#bbb' }} />
                        </div>
                        <Select
                            style={{ width: 90 }}
                            value={viewer}
                            onChange={this.onChangeViewer}
                            options={this.constructJSONOption(obj)}
                        />
                    </div>
                    <div className="instance-value-display">
                        <div
                            className="instance-value-content"
                            style={{ fontFamily: "'Courier New', Courier, monospace", fontSize: 13, lineHeight: 1.5 }}
                        >
                            {!currentKey && (
                                <div style={{ color: '#999' }}>
                                    Select a key to view its value
                                </div>
                            )}
                            {currentKey && viewer === "RAW" && obj && obj.value}
                            {currentKey && viewer === "JSON" && obj && obj.value && (
                                <pre style={{ margin: 0 }}>{JSON.stringify(JSON.parse(obj.value), null, 2)}</pre>
                            )}
                        </div>
                    </div>
                </div>

                <Modal
                    title={kvModalMode === 'edit' ? 'Edit Key/Value' : 'Add Key/Value'}
                    open={kvModalOpen}
                    onOk={this.submitKeyValue}
                    onCancel={this.closeKeyValueModal}
                    okText={kvModalMode === 'edit' ? 'Update' : 'Add'}
                >
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                        <div>
                            <div style={{ marginBottom: 6 }}>Key</div>
                            <Input
                                value={kvKeyInput}
                                onChange={this.onChangeKVKey}
                                placeholder="Enter key"
                                disabled={kvModalMode === 'edit'}
                            />
                        </div>
                        <div>
                            <div style={{ marginBottom: 6 }}>Value</div>
                            <Input.TextArea
                                value={kvValueInput}
                                onChange={this.onChangeKVValue}
                                autoSize={{ minRows: 3, maxRows: 10 }}
                                placeholder="Enter value"
                            />
                        </div>
                    </div>
                </Modal>
            </div>
        );
    }

    componentDidMount() {
        this.props.getConfigByKey("databases");
        // Ensure default DB0 and load keys
        this.props.changeDataBase(0);
    }

    componentDidUpdate(prevProps) {
        // After keys loaded / db switched, default select the first key
        if (this.props.keys !== prevProps.keys) {
            const keys = this.props.keys || [];
            if (keys.length > 0) {
                const firstKey = keys[0];
                if (this.state.currentKey !== firstKey) {
                    this.setState({ currentKey: firstKey });
                    this.props.getObjectByKey(firstKey);
                }
            } else if (this.state.currentKey) {
                this.setState({ currentKey: null });
            }
        }

        // If config (databases count) arrives, ensure DB select is valid and defaults to first
        if (this.props.config !== prevProps.config) {
            const total = this.props.config ? parseInt(this.props.config.value) : 0;
            if (total > 0) {
                // keep current selection if valid, else reset to DB0
                const idx = parseInt(String(this.state.selectedDB).replace('DB', ''), 10);
                if (Number.isNaN(idx) || idx < 0 || idx >= total) {
                    this.setState({ selectedDB: 'DB0' });
                    this.props.changeDataBase(0);
                }
            }
        }
    }
}

function mapStateToProps(state) {
    return {
        obj: state.handleRedis.obj || null,
        config: state.handleRedis.config || null,
        keys: state.handleRedis.keys || null,
        instance: state.handleConnection.client || null,
        isChanging: state.handleRedis.isChanging || null,
        db: state.handleRedis.db || null
    }
}

export default connect(mapStateToProps, {
    getConfigByKey, setConfigByKey,
    getCurrentInstanceKeys, getObjectByKey,
    changeDataBase,
    setKeyValue,
    deleteKey
})(Instance);
