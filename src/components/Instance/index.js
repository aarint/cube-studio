/**
 * The DB instance will be show in new tab.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Button, Layout, Tree, notification, List, Select, Row, Col } from 'antd';
import { DeleteOutlined, EditOutlined } from '@ant-design/icons';
import { getCurrentInstanceKeys, getConfigByKey, setConfigByKey, getObjectByKey, changeDataBase } from '../../redux/thunk/Redis';

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
        viewer: 'RAW'
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
                style={{ border: '1px solid #f0f0f0', borderRadius: 4, height: 'calc(100vh - 200px)', overflowY: 'auto' }}
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
        const db = value.substring(2);
        this.props.changeDataBase(db);
    }

    constructJSONOption = (obj) => {
        const options = [{ key: 'RAW', value: 'RAW', label: 'RAW' }];

        if (obj && obj.value && isJSON(obj.value)) {
            options.push({ key: 'JSON', value: 'JSON', label: 'JSON' });
        }

        return options;
    }

    render() {
        const { currentKey, viewer } = this.state;
        const { instance, keys, config, obj } = this.props;

        return (
            <Row style={{ position: 'absolute', width: '100%', top: 60, bottom: 22, padding: 10, border: 'solid 1px silver' }}>
                <Col span={8} style={{ background: 'white', borderRight: 'solid 1px silver' }}>
                    <div style={{ padding: 16, borderBottom: '1px solid #f0f0f0' }}>
                        <div style={{ marginBottom: 12, fontWeight: 500 }}>Redis DB</div>
                        <Select style={{ width: '100%' }} defaultValue="DB0" onSelect={this.onSelectDB} options={this.constructOptions()} />
                    </div>
                    {this.constructKeys()}
                </Col>
                <Col span={16}>
                    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', background: '#fafafa' }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '12px 16px', background: '#ffffff', borderBottom: '1px solid #f0f0f0' }}>
                            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                                {obj && <span style={{ fontSize: 14 }}>{obj.type}</span>}
                                <DeleteOutlined onClick={() => this.addStr()} />
                                <EditOutlined onClick={() => this.getStr()} />
                            </div>
                            <Select style={{ width: 90 }} defaultValue={"RAW"} onChange={this.onChangeViewer} options={this.constructJSONOption(obj)} />
                        </div>
                        <div style={{ flex: 1, overflow: 'hidden' }}>
                            <div style={{ height: '100%', padding: 16, overflowY: 'auto', background: 'white', wordBreak: 'break-all', fontFamily: "'Courier New', Courier, monospace", fontSize: 13, lineHeight: 1.5 }}>
                                {viewer === "RAW" && obj && obj.value}
                                {viewer === "JSON" && (
                                    <pre>{obj && obj.value && (JSON.stringify(JSON.parse(obj.value), null, 2))}</pre>
                                )}
                            </div>
                        </div>
                    </div>
                </Col>
            </Row>
        );
    }

    componentDidMount() {
        this.props.getConfigByKey("databases");
        this.props.getCurrentInstanceKeys();
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
    changeDataBase
})(Instance);
