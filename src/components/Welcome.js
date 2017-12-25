/**
 * Main page show the all connections that include saved in storage.
 * Click one will show the DB instance in the opened tab window.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Modal, Button, Layout } from 'antd';
import Instance from './Instance';
import { connectDB, connectDBDone } from '../redux/actions/Connect';
import { addInstance, getInstance, getConfig } from '../redux/actions/Instance';

const { Content, Sider } = Layout;

class Welcome extends React.PureComponent {

    state = {
        visible: false
    }

    /**
     * Open connection dialog. Connect the DB after input the connection config.
     * Then create a new tab that contain the DB instance.
     */
    openConnection = () => {
        this.setState({ visible: true })

        //dialog.showOpenDialog({ properties: ['openFile', 'openDirectory', 'multiSelections'] })
    }

    handleOK = () => {
        this.props.addInstance(this.props.instance);
    }

    render() {
        return (
            <Layout>
                <Content style={{ marginLeft: '200px', overflow: 'initial' }}>
                    <div style={{ padding: 0, background: '#fff', textAlign: 'center' }}>
                        <div></div>
                        <Button type="primary" onClick={this.openConnection}>open connect dialog</Button>
                    </div>
                </Content>

                {
                    this.state.visible &&
                    <div>
                        <Button onClick={this.props.connectDB}>connect</Button>
                    </div>
                }

                {

                }
            </Layout>
        )
    }

    componentDidUpdate() {

    }
}

export default connect(state => {
    return {
        connected: state.handleConnection.connected,
        instance: state.handleConnection.client
    }
}, { connectDB, addInstance })(Welcome)

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