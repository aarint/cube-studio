/**
 * The DB instance will be show in new tab.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Button, Layout, Tree, notification, List, Select, Modal, Input, Popconfirm, Tooltip } from 'antd';
import { PlusOutlined, EditOutlined, DeleteOutlined } from '@ant-design/icons';
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
        kvValueInput: '',
        isEditingValue: false,
        editDraftValue: '',
        editValueIsJson: false
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
        this.setState({ currentKey: key, isEditingValue: false, editDraftValue: '', editValueIsJson: false });
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
        const raw = String(obj.value ?? '');
        const valueIsJson = isJSON(raw);
        const draft = valueIsJson
            ? JSON.stringify(JSON.parse(raw), null, 2)
            : raw;
        this.setState({ isEditingValue: true, editDraftValue: draft, editValueIsJson: valueIsJson });
    }

    exitEditValue = () => {
        this.setState({ isEditingValue: false, editDraftValue: '', editValueIsJson: false });
    }

    onChangeEditDraft = (e) => {
        this.setState({ editDraftValue: e.target.value });
    }

    saveEditValue = () => {
        const { currentKey, editDraftValue, editValueIsJson } = this.state;
        if (!currentKey) return;
        let valueToSave = editDraftValue;
        if (editValueIsJson) {
            try {
                valueToSave = JSON.stringify(JSON.parse(editDraftValue));
            } catch (e) {
                notification.error({ message: 'Invalid JSON', description: e.message });
                return;
            }
        }
        return this.props.setKeyValue(currentKey, valueToSave)
            .then(() => {
                this.exitEditValue();
                this.props.getObjectByKey(currentKey);
                notification.success({ message: 'Saved' });
            })
            .catch((err) => {
                notification.error({
                    message: 'Save failed',
                    description: (err && (err.message || (err.toString && err.toString()))) || 'Unknown error'
                });
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
        const { currentKey, viewer, selectedDB, kvModalOpen, kvModalMode, kvKeyInput, kvValueInput, isEditingValue, editDraftValue, editValueIsJson } = this.state;
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
                    </div>
                    <div className="instance-keys-list">
                        {this.constructKeys()}
                    </div>
                </div>
                <div className="instance-content">
                    <div className="instance-top-bar">
                        <div className="instance-actions" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                            <Select
                                style={{ width: 90 }}
                                value={viewer}
                                onChange={this.onChangeViewer}
                                options={this.constructJSONOption(obj)}
                            />
                            {obj && <span style={{ fontSize: 14, color: '#666' }}>{obj.type}</span>}
                            <Tooltip title="Add Key/Value">
                                <Button type="primary" size="small" icon={<PlusOutlined />} onClick={this.openAddKeyValue} />
                            </Tooltip>
                            <Tooltip title="Edit">
                                <Button size="small" icon={<EditOutlined />} onClick={this.openEditKeyValue} disabled={!currentKey} />
                            </Tooltip>
                            <Tooltip title="Delete">
                                <span>
                                    <Popconfirm
                                        title="Delete this key?"
                                        okText="Delete"
                                        cancelText="Cancel"
                                        onConfirm={this.onDeleteKey}
                                    >
                                        <Button size="small" icon={<DeleteOutlined />} disabled={!currentKey} />
                                    </Popconfirm>
                                </span>
                            </Tooltip>
                        </div>
                    </div>
                    {isEditingValue ? (
                        <>
                            <div className="instance-value-display" style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
                                <div style={{ flex: 1, padding: 12, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
                                    {editValueIsJson ? (
                                        <Input.TextArea
                                            value={editDraftValue}
                                            onChange={this.onChangeEditDraft}
                                            placeholder='{ }'
                                            style={{
                                                flex: 1,
                                                fontFamily: "'Courier New', Courier, monospace",
                                                fontSize: 13,
                                                lineHeight: 1.5,
                                                minHeight: 120
                                            }}
                                            className="instance-json-editor"
                                        />
                                    ) : (
                                        <Input.TextArea
                                            value={editDraftValue}
                                            onChange={this.onChangeEditDraft}
                                            placeholder="Value"
                                            style={{
                                                flex: 1,
                                                fontFamily: "'Courier New', Courier, monospace",
                                                fontSize: 13,
                                                lineHeight: 1.5,
                                                minHeight: 120
                                            }}
                                        />
                                    )}
                                </div>
                            </div>
                            <div className="instance-edit-bar" style={{ padding: '8px 16px', borderTop: '1px solid #f0f0f0', background: '#fff', display: 'flex', gap: 8 }}>
                                <Button type="primary" size="small" onClick={this.saveEditValue}>Save</Button>
                                <Button size="small" onClick={this.exitEditValue}>Cancel</Button>
                            </div>
                        </>
                    ) : (
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
                                {currentKey && !isEditingValue && viewer === "RAW" && (obj?.value !== undefined) && (
                                    <span>{obj.value === '' ? '(empty)' : obj.value}</span>
                                )}
                                {currentKey && !isEditingValue && viewer === "JSON" && obj?.value != null && (
                                    isJSON(obj.value) ? (
                                        <pre style={{ margin: 0 }}>{JSON.stringify(JSON.parse(obj.value), null, 2)}</pre>
                                    ) : (
                                        <span>{obj.value}</span>
                                    )
                                )}
                            </div>
                        </div>
                    )}
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
