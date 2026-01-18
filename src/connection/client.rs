/// Client connection state
pub struct Client {
    pub conn: quiche::Connection,
    pub h3_conn: Option<quiche::h3::Connection>,
}

impl Client {
    pub fn new(conn: quiche::Connection) -> Self {
        Self {
            conn,
            h3_conn: None,
        }
    }

    /// Try to create HTTP/3 connection if not already created
    pub fn try_create_h3(&mut self) -> Result<bool, quiche::h3::Error> {
        if self.conn.is_established() && self.h3_conn.is_none() {
            let mut h3_config = quiche::h3::Config::new()?;
            h3_config.set_max_field_section_size(16384);
            h3_config.set_qpack_max_table_capacity(100);
            h3_config.set_qpack_blocked_streams(100);

            match quiche::h3::Connection::with_transport(&mut self.conn, &h3_config) {
                Ok(h3_conn) => {
                    self.h3_conn = Some(h3_conn);
                    log::info!("HTTP/3 connection established");
                    Ok(true)
                }
                Err(quiche::h3::Error::Done) => Ok(false),
                Err(e) => Err(e),
            }
        } else {
            Ok(false)
        }
    }
}
