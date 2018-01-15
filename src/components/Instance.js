/**
 * The DB instance will be show in new tab.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Button, Layout, Tree, notification, List, Select } from 'antd';
import { addInstance, getInstance } from '../redux/thunk/Instance';
import { getCurrentInstanceKeys, getConfigByKey, setConfig, getObjectByKey } from '../redux/thunk/Redis';

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
        for (let i = 1; i <= parseInt(config.value); i++) {
            options.push(<Option key={`${i}`} value={`DB${i}`}>{`DB${i}`}</Option>)
        }

        return options;
    }

    constructKeys() {
        const { keys } = this.props;

        if (!keys) {
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
            <Layout style={styles.container}>
                <Sider style={styles.sider}>
                    <div style={{ height: 30, paddingLeft: 10 }}>Redis DB</div>
                    <Select style={{ width: '100%' }} defaultValue="DB1">
                        {this.constructOptions()}
                    </Select>
                    <ul>
                        {this.constructKeys()}
                    </ul>
                </Sider>
                <Content>
                    <div style={styles.topBar}>
                        <div style={{ float: 'left' }}> {obj && obj.type} </div>
                        <Select style={{ width: 90, float: 'right' }} defaultValue={"RAW"} onChange={this.onChangeViewer}>
                            <Option key='RAW'>RAW</Option>
                            {
                                this.constructJSONOption(obj)
                            }

                        </Select>
                    </div>
                    <div style={{ width: '100%', height: '100%' }}>
                        <div style={{ width: '100%', height: '300px' }}>
                            {viewer === "RAW" && obj && obj.value}

                            {
                                viewer === "JSON" && (
                                    <pre>{obj && obj.value && (JSON.stringify(JSON.parse(obj.value), null, 2))}</pre>
                                )
                            }

                        </div>
                    </div>
                    <div style={styles.topBar}>
                        <Button type="primary" onClick={() => this.addStr()}>Delete</Button>
                        <Button type="primary" onClick={() => this.getStr()}>Update</Button>
                    </div>
                </Content>
            </Layout>
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
        instance: state.handleConnection.client || null
    }
}

export default connect(mapStateToProps, { getConfigByKey, setConfig, getCurrentInstanceKeys, getObjectByKey })(Instance);

const styles = {
    container: {
        overflow: 'hidden',
        width: '100%'
    },
    topBar: {
        width: '100%',
        height: '32px'
    },
    sider: {
        overflow: 'auto',
        left: 0,
        border: 'solid 1 silver',
        background: "white"
    },
    key: {
        borderBottom: 'solid 1px silver',
        listStyleType: 'none'
    }
}