/**
 * The DB instance will be show in new tab.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Button, Layout, notification } from 'antd';
import { actionConnect } from '../redux/actions/Connect';
import { getInstance, getConfig } from '../redux/actions/Instance';
import { doingString, addString, getString } from '../redux/actions/Redis';

const { Content, Sider } = Layout;

class Instance extends React.PureComponent {
    constructor(props) {
        super(props)

        this.curInstance = this.props.curInstance;
    }

    connectDB() {
        console.log('instances');
        this.props.actionConnect();
    }

    addString() {
        this.props.addString('test', 'test string...');

        notification.open({ message: "Did add a string successfully!!!" });
    }

    getString() {
        this.props.getString('test');

        notification.open({ message: "Get a string!!!" });
    }

    render() {
        return (
            <div>
                <Sider style={styles.sider}>
                    <div>shit sider</div>
                </Sider>
                <Button type="primary" onClick={() => this.connectDB()}>Test Connection</Button>
                <Button type="primary" onClick={() => this.connectDB()}>Connect</Button>
                <Button type="primary" onClick={() => this.addString()}>Add a string.</Button>
                <Button type="primary" onClick={() => this.getString()}>Get a string.</Button>
            </div>
        )
    }

    componentDidMount() {
        console.log('component did mount');

        this.props.getConfig();
        this.props.getInstance();
    }
}

function mapStateToProps(state) {
    console.log(state);

    return {
        config: state.handleInstance.config
    }
}

export default connect(mapStateToProps, { actionConnect, getConfig, getInstance, addString, getString })(Instance);

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