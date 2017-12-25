/**
 * The DB instance will be show in new tab.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Button, Layout, notification } from 'antd';
import { connectDB, connectDBDone } from '../redux/actions/Connect';
import { addInstance, getInstance, getConfig } from '../redux/actions/Instance';
import { doingString, addString, getString, getAllKeys } from '../redux/actions/Redis';

const { Content, Sider } = Layout;

class Instance extends React.PureComponent {
    constructor(props) {
        super(props)

        this.curInstance = this.props.curInstance;
    }

    /**
     * TODO:
     * These code will be moved to the welcome component.
     * The connection operation will be finished after inputting the login information.
     * 
     */
    connectDB = () => {

    }

    addString() {
        this.props.addString('test', 'test string...');

        notification.open({ message: "Did add a string successfully!!!" });
    }

    getString() {
        this.props.getString('test');

        notification.open({ message: "Get a string!!!" });
    }

    getAllKeys() {
        this.props.getAllKeys();
    }

    render() {
        const { instance, keys } = this.props;
        console.log(instance, keys);

        return (
            <div>
                <Sider style={styles.sider}>
                    <div>shit sider</div>
                </Sider>
                <Button type="primary" onClick={() => this.connectDB()}>Test Connection</Button>
                <Button type="primary" onClick={() => this.connectDB()}>Connect</Button>
                <Button type="primary" onClick={() => this.addString()}>Add a string.</Button>
                <Button type="primary" onClick={() => this.getString()}>Get a string.</Button>
                <Button type="primary" onClick={() => this.getAllKeys()}>Get all keys.</Button>
            </div>
        )
    }

    componentDidMount() {
        this.props.getConfig();
        this.props.addInstance();
    }
}

function mapStateToProps(state) {
    console.log(state);

    return {
        keys: state.handleInstance.keys,
        instance: state.handleConnection.client
    }
}

export default connect(mapStateToProps, {
    connectDB, connectDBDone,
    getConfig,
    addInstance, getInstance,
    addString, getString,
    getAllKeys
})(Instance);

const styles = {
    sider: {
        color: 'green',
        overflow: 'auto',
        height: '100vh',
        position: 'fixed',
        left: 0,
        border: 'solid 1 silver',
        background: "black"
    }
}