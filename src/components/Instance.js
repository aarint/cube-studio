/**
 * The DB instance will be show in new tab.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Button, Layout, Tree, notification, List, Select, Row, Col, Icon } from 'antd';
import { addInstance, getInstance } from '../redux/thunk/Instance';
import { getCurrentInstanceKeys, getConfigByKey, setConfig, getObjectByKey, changeDataBase } from '../redux/thunk/Redis';

require('json-editor');

const { Content, Sider } = Layout;
const TreeNode = Tree.TreeNode;
const Option = Select.Option;

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
        super(props)

        this.curInstance = this.props.curInstance;
    }

    constructDBTree() {
        const { config } = this.props;

        if (!config) {
            return;
        }

        let treeNodes = [];
        for (let i = 1; i <= parseInt(config.value); i++) {
            treeNodes.push(<TreeNode title={`DB${i}`} key={`${i}`}></TreeNode>);
        }

        return treeNodes;
    }

    constructOptions() {
        const { config, obj } = this.props;

        if (!config) {
            return;
        }

        let options = [];
        for (let i = 0; i < parseInt(config.value); i++) {
            options.push(<Option key={`${i}`} value={`DB${i}`}>{`DB${i}`}</Option>)
        }

        return options;
    }

    constructKeys() {
        const { keys, isChanging } = this.props;

        if (!keys || isChanging) {
            return;
        }

        let items = [];
        for (let i = 0; i < keys.length; i++) {
            items.push(<li style={styles.key} key={`${i}`} onClick={() => this.onSelectKey(`${keys[i]}`)}><span></span><span>{`${keys[i]}`}</span></li>)
        }

        return items;
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
        if (obj && obj.value && isJSON(obj.value)) {
            return <Option key='JSON'>JSON</Option>
        }

        return <Option key='JSON' disabled>JSON</Option>
    }

    render() {
        const { currentKey, viewer } = this.state;
        const { instance, keys, config, obj } = this.props;

        return (
            <Row style={{ position: 'absolute',width:'100%',top:60, bottom: 22, padding: 10,bakcground:'red',border:'solid 1px silver' }}>
                <Col span={8} style={{ background: 'white', borderRight: 'solid 1px silver' }}>
                    <Sider style={styles.sider}>
                        <div style={{ height: 30, paddingLeft: 10 }}>Redis DB</div>
                        <Select style={{ width: '100%' }} defaultValue="DB0" onSelect={this.onSelectDB}>
                            {this.constructOptions()}
                        </Select>
                        <ul>
                            {this.constructKeys()}
                        </ul>
                    </Sider>
                </Col>
                <Col span={16}>
                    <Content>
                        <div style={styles.topBar}>
                            <div style={{ float: 'left', fontSize: 16, width: '200px' }}>
                                {obj && <label style={{ color: '#FFFFFF', background: 'green', }}>{obj.type}&nbsp;</label>}
                                <Icon type='delete' onClick={() => this.addStr()} />
                                <Icon type='edit' onClick={() => this.getStr()} />
                            </div>
                            <Select style={{ width: 90, float: 'right' }} defaultValue={"RAW"} onChange={this.onChangeViewer}>
                                <Option key='RAW'>RAW</Option>
                                {
                                    this.constructJSONOption(obj)
                                }

                            </Select>
                        </div>
                        <div style={{ height: '100%', background: 'yellow' }}>
                            <div style={{ height: '300px', overflowY: 'scroll', wordBreak: 'break-all' }}>
                                {viewer === "RAW" && obj && obj.value}
                                {
                                    viewer === "JSON" && (
                                        <pre>{obj && obj.value && (JSON.stringify(JSON.parse(obj.value), null, 2))}</pre>
                                    )
                                }

                            </div>
                        </div>

                    </Content>
                </Col>
            </Row>



        )
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
    getConfigByKey, setConfig,
    getCurrentInstanceKeys, getObjectByKey,
    changeDataBase
})(Instance);

const styles = {
    container: {
        position: 'relative',
        top: 0,
        bottom: 22,
        background: 'red',
        width: '100%'
    },
    topBar: {
        width: '100%',
        height: '32px',
        paddingLeft: 5
    },
    sider: {
        position: 'relative',
        top: 0,
        bottom: 0,
        left: 0,
        border: 'solid 1 silver',
        background: "white"
    },
    key: {
        borderBottom: 'solid 1px silver',
        listStyleType: 'none'
    }
}