/**
 * The DB instance will be show in new tab.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Button, Layout } from 'antd';
import { actionConnect } from '../redux/actions/Connect';
import {} from '../redux/actions/Instance';

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

    render() {
        return (
            <div>
                <Sider style={styles.sider}>
                    <div>shit sider</div>
                </Sider>
                <Button type="primary" onClick={() => this.connectDB()}>Test Connection</Button>
                <Button type="primary" onClick={() => this.connectDB()}>Connect</Button>
            </div>
        )
    }
}

function mapStateToProps(state) {
    return {}
}

export default connect(mapStateToProps, { actionConnect })(Instance);

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