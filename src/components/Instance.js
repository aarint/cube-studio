/**
 * The DB instance will be show in new tab.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Button, Layout, Tree, notification, List, Select } from 'antd';
import { connectDB, connectDBDone } from '../redux/actions/Connect';
import { addInstance, getInstance } from '../redux/actions/Instance';
import { doingString, getAllKeys, getConfig, setConfig, getKeyValue } from '../redux/actions/Redis';

const { Content, Sider } = Layout;
const TreeNode = Tree.TreeNode;
const Option = Select.Option;

class Instance extends React.PureComponent {
    state = {
        currentKey: null,
        currentValue: null
    }

    constructor(props) {
        super(props)

        this.curInstance = this.props.curInstance;
    }

    getStr() {
        //this.props.getString('test');
        //notification.open({ message: "Get a string!!!" });
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
        const { config } = this.props;

        if (!config) {
            return;
        }

        let options = [];
        for (let i = 1; i <= parseInt(config.value); i++) {
            options.push(<Option key={`${i}`} value={`${i}`}>{`DB${i}`}</Option>)
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
            items.push(<li key={`${i}`} onClick={() => this.onSelectKey(`${keys[i]}`)}><span></span><span>{`${keys[i]}`}</span></li>)
        }

        return items;
    }

    onSelectKey = (key) => {
        this.setState({ currentKey: key });
        this.props.getKeyValue(key);
    }

    onSelectNode = (keys, info) => {
        console.log(keys, info);
    }



    render() {
        const { currentKey } = this.state;
        const { instance, keys, config, obj } = this.props;

        console.log(obj);

        return (
            <Layout>
                <Sider style={styles.sider}>
                    <Select style={{ width: '100%' }} >
                        {this.constructOptions()}
                    </Select>
                    <Tree defaultExpandAll={true}
                        onSelect={this.onSelectNode}>
                        <TreeNode title="Redis" key="0">
                            {this.constructDBTree()}
                        </TreeNode>
                    </Tree>
                    <ul style={{background:'green'}}>
                        {this.constructKeys()}
                    </ul>
                </Sider>
                <Content style={{ marginLeft: '200px', overflow: 'initial' }}>
                    <div>
                        <Button type="primary" onClick={() => this.addStr()}>Add a string.</Button>
                        <Button type="primary" onClick={() => this.getStr()}>Get a string.</Button>
                    </div>
                    <div style={{ width: 200, background: 'blue' }}>
                        <ul>{this.constructKeys()}</ul>
                    </div>
                    <div style={{ width: 300, background: 'green' }}>
                        <div> {obj && obj.value} </div>
                    </div>
                </Content>
            </Layout>
        )
    }

    componentDidMount() {
        this.props.getConfig("databases");
        this.props.getAllKeys();
    }
}

function mapStateToProps(state) {
    console.log(state);

    return {
        obj: state.handleRedis.obj || null,
        config: state.handleRedis.config || null,
        keys: state.handleRedis.keys || null,
        instance: state.handleConnection.client || null
    }
}

export default connect(mapStateToProps, { getConfig, setConfig, getAllKeys, getKeyValue })(Instance);

const styles = {
    sider: {
        color: 'green',
        overflow: 'auto',
        height: '100vh',
        position: 'fixed',
        left: 0,
        border: 'solid 1 silver',
        background: "white"
    }
}