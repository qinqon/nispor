use crate::ifaces::bond::get_bond_info;
use crate::ifaces::bond::get_bond_subordinate_info;
use crate::ifaces::bond::BondInfo;
use crate::ifaces::bond::BondSubordinateInfo;
use crate::ifaces::bridge::get_bridge_info;
use crate::ifaces::bridge::get_bridge_port_info;
use crate::ifaces::bridge::parse_bridge_vlan_info;
use crate::ifaces::bridge::BridgeInfo;
use crate::ifaces::bridge::BridgePortInfo;
use crate::ifaces::sriov::get_sriov_info;
use crate::ifaces::sriov::SriovInfo;
use crate::ifaces::veth::VethInfo;
use crate::ifaces::vlan::get_vlan_info;
use crate::ifaces::vlan::VlanInfo;
use crate::ifaces::vrf::get_vrf_info;
use crate::ifaces::vrf::get_vrf_subordinate_info;
use crate::ifaces::vrf::VrfInfo;
use crate::ifaces::vrf::VrfSubordinateInfo;
use crate::ifaces::vxlan::get_vxlan_info;
use crate::ifaces::vxlan::VxlanInfo;
use crate::Ipv4Info;
use crate::Ipv6Info;
use netlink_packet_route::rtnl::link::nlas;
use netlink_packet_route::rtnl::LinkMessage;
use netlink_packet_route::rtnl::{
    ARPHRD_ETHER, IFF_ALLMULTI, IFF_AUTOMEDIA, IFF_BROADCAST, IFF_DEBUG,
    IFF_DORMANT, IFF_LOOPBACK, IFF_LOWER_UP, IFF_MASTER, IFF_MULTICAST,
    IFF_NOARP, IFF_POINTOPOINT, IFF_PORTSEL, IFF_PROMISC, IFF_RUNNING,
    IFF_SLAVE, IFF_UP,
};

use rtnetlink::packet::rtnl::link::nlas::Nla;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum IfaceType {
    Bond,
    Veth,
    Bridge,
    Vlan,
    Dummy,
    Vxlan,
    Loopback,
    Ethernet,
    Vrf,
    Unknown,
    Other(String),
}

impl Default for IfaceType {
    fn default() -> Self {
        IfaceType::Unknown
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum IfaceState {
    Up,
    Dormant,
    Down,
    LowerLayerDown,
    Other(String),
    Unknown,
}

impl Default for IfaceState {
    fn default() -> Self {
        IfaceState::Unknown
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum IfaceFlags {
    AllMulti,
    AutoMedia,
    Broadcast,
    Debug,
    Dormant,
    Loopback,
    LowerUp,
    Controller,
    Multicast,
    #[serde(rename = "NOARP")]
    NoArp,
    PoinToPoint,
    Portsel,
    Promisc,
    Running,
    Subordinate,
    Up,
    Other(u32),
    Unknown,
}

impl Default for IfaceFlags {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ControllerType {
    Bond,
    Bridge,
    Vrf,
    Other(String),
    Unknown,
}

impl From<&str> for ControllerType {
    fn from(s: &str) -> Self {
        match s {
            "bond" => ControllerType::Bond,
            "bridge" => ControllerType::Bridge,
            "vrf" => ControllerType::Vrf,
            _ => ControllerType::Other(s.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub struct Iface {
    pub name: String,
    #[serde(skip_serializing)]
    pub index: u32,
    pub iface_type: IfaceType,
    pub state: IfaceState,
    pub mtu: i64,
    pub flags: Vec<IfaceFlags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv4: Option<Ipv4Info>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6: Option<Ipv6Info>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mac_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller_type: Option<ControllerType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bond: Option<BondInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bond_subordinate: Option<BondSubordinateInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge: Option<BridgeInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_port: Option<BridgePortInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan: Option<VlanInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vxlan: Option<VxlanInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub veth: Option<VethInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vrf: Option<VrfInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vrf_subordinate: Option<VrfSubordinateInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sriov: Option<SriovInfo>,
}

pub(crate) fn get_iface_name_by_index(
    iface_states: &HashMap<String, Iface>,
    iface_index: u32,
) -> String {
    for (iface_name, iface) in iface_states.iter() {
        if iface.index == iface_index {
            return iface_name.clone();
        }
    }
    "".into()
}

pub(crate) fn parse_nl_msg_to_iface(nl_msg: &LinkMessage) -> Option<Iface> {
    let name = _get_iface_name(&nl_msg);
    if name.len() <= 0 {
        return None;
    }
    let mut iface_state = Iface {
        name: name.clone(),
        ..Default::default()
    };
    if nl_msg.header.link_layer_type == ARPHRD_ETHER {
        iface_state.iface_type = IfaceType::Ethernet
    }
    iface_state.index = nl_msg.header.index;
    let mut link: Option<u32> = None;
    let mut mac_len = None;
    for nla in &nl_msg.nlas {
        if let Nla::Mtu(mtu) = nla {
            iface_state.mtu = *mtu as i64;
        } else if let Nla::Address(mac) = nla {
            mac_len = Some(mac.len());
            let mut mac_str = String::new();
            for octet in mac.iter() {
                mac_str.push_str(&format!("{:02X?}:", octet));
            }
            mac_str.pop();
            iface_state.mac_address = mac_str;
        } else if let Nla::OperState(state) = nla {
            iface_state.state = _get_iface_state(&state);
        } else if let Nla::Master(controller) = nla {
            iface_state.controller = Some(format!("{}", controller));
        } else if let Nla::Link(l) = nla {
            link = Some(*l);
        } else if let Nla::Info(infos) = nla {
            for info in infos {
                if let nlas::Info::Kind(t) = info {
                    iface_state.iface_type = match t {
                        nlas::InfoKind::Bond => IfaceType::Bond,
                        nlas::InfoKind::Veth => IfaceType::Veth,
                        nlas::InfoKind::Bridge => IfaceType::Bridge,
                        nlas::InfoKind::Vlan => IfaceType::Vlan,
                        nlas::InfoKind::Vxlan => IfaceType::Vxlan,
                        nlas::InfoKind::Dummy => IfaceType::Dummy,
                        nlas::InfoKind::Vrf => IfaceType::Vrf,
                        nlas::InfoKind::Other(s) => IfaceType::Other(s.clone()),
                        _ => IfaceType::Other(format!("{:?}", t)),
                    };
                }
            }
            for info in infos {
                if let nlas::Info::Data(d) = info {
                    match iface_state.iface_type {
                        IfaceType::Bond => iface_state.bond = get_bond_info(&d),
                        IfaceType::Bridge => {
                            iface_state.bridge = get_bridge_info(&d)
                        }
                        IfaceType::Vlan => iface_state.vlan = get_vlan_info(&d),
                        IfaceType::Vxlan => {
                            iface_state.vxlan = get_vxlan_info(&d)
                        }
                        IfaceType::Vrf => iface_state.vrf = get_vrf_info(&d),
                        _ => eprintln!(
                            "Unhandled iface type {:?}",
                            iface_state.iface_type
                        ),
                    }
                }
            }
            for info in infos {
                if let nlas::Info::SlaveKind(d) = info {
                    // Remove the tailing \0
                    match std::str::from_utf8(&(d.as_slice()[0..(d.len() - 1)]))
                    {
                        Ok(controller_type) => {
                            iface_state.controller_type =
                                Some(controller_type.into())
                        }
                        _ => (),
                    }
                }
            }
            if let Some(controller_type) = &iface_state.controller_type {
                for info in infos {
                    if let nlas::Info::SlaveData(d) = info {
                        match controller_type {
                            ControllerType::Bond => {
                                iface_state.bond_subordinate =
                                    get_bond_subordinate_info(&d);
                            }
                            ControllerType::Bridge => {
                                iface_state.bridge_port =
                                    get_bridge_port_info(&d);
                            }
                            ControllerType::Vrf => {
                                iface_state.vrf_subordinate =
                                    get_vrf_subordinate_info(&d);
                            }
                            _ => eprintln!(
                                "Unknown controller type {:?}",
                                controller_type
                            ),
                        }
                    }
                }
            }
        } else if let Nla::VfInfoList(data) = nla {
            if let Ok(info) = get_sriov_info(data, mac_len) {
                iface_state.sriov = Some(info);
            }
        } else {
            // println!("{} {:?}", name, nla);
        }
    }
    if let Some(old_vlan_info) = &iface_state.vlan {
        if let Some(base_iface_index) = link {
            let mut new_vlan_info = old_vlan_info.clone();
            new_vlan_info.base_iface = format!("{}", base_iface_index);
            iface_state.vlan = Some(new_vlan_info);
        }
    }
    if let Some(iface_index) = link {
        if iface_state.iface_type == IfaceType::Veth {
            iface_state.veth = Some(VethInfo {
                peer: format!("{}", iface_index),
            })
        }
    }
    if (nl_msg.header.flags & IFF_LOOPBACK) > 0 {
        iface_state.iface_type = IfaceType::Loopback;
    }
    iface_state.flags = _parse_iface_flags(nl_msg.header.flags);
    Some(iface_state)
}

fn _get_iface_name(nl_msg: &LinkMessage) -> String {
    for nla in &nl_msg.nlas {
        if let Nla::IfName(name) = nla {
            return name.clone();
        }
    }
    "".into()
}

pub(crate) fn fill_bridge_vlan_info(
    iface_states: &mut HashMap<String, Iface>,
    nl_msg: &LinkMessage,
) {
    let name = _get_iface_name(&nl_msg);
    if name.len() <= 0 {
        return;
    }
    if let Some(mut iface_state) = iface_states.get_mut(&name) {
        for nla in &nl_msg.nlas {
            if let Nla::AfSpecBridge(data) = nla {
                parse_bridge_vlan_info(&mut iface_state, &data);
                break;
            }
        }
    }
}

fn _get_iface_state(state: &nlas::State) -> IfaceState {
    match state {
        nlas::State::Up => IfaceState::Up,
        nlas::State::Dormant => IfaceState::Dormant,
        nlas::State::Down => IfaceState::Down,
        nlas::State::LowerLayerDown => IfaceState::LowerLayerDown,
        nlas::State::Unknown => IfaceState::Unknown,
        _ => IfaceState::Other(format!("{:?}", state)),
    }
}

fn _parse_iface_flags(flags: u32) -> Vec<IfaceFlags> {
    let mut ret = Vec::new();
    if (flags & IFF_ALLMULTI) > 0 {
        ret.push(IfaceFlags::AllMulti)
    }
    if (flags & IFF_AUTOMEDIA) > 0 {
        ret.push(IfaceFlags::AutoMedia)
    }
    if (flags & IFF_BROADCAST) > 0 {
        ret.push(IfaceFlags::Broadcast)
    }
    if (flags & IFF_DEBUG) > 0 {
        ret.push(IfaceFlags::Debug)
    }
    if (flags & IFF_DORMANT) > 0 {
        ret.push(IfaceFlags::Dormant)
    }
    if (flags & IFF_LOOPBACK) > 0 {
        ret.push(IfaceFlags::Loopback)
    }
    if (flags & IFF_LOWER_UP) > 0 {
        ret.push(IfaceFlags::LowerUp)
    }
    if (flags & IFF_MASTER) > 0 {
        ret.push(IfaceFlags::Controller)
    }
    if (flags & IFF_MULTICAST) > 0 {
        ret.push(IfaceFlags::Multicast)
    }
    if (flags & IFF_NOARP) > 0 {
        ret.push(IfaceFlags::NoArp)
    }
    if (flags & IFF_POINTOPOINT) > 0 {
        ret.push(IfaceFlags::PoinToPoint)
    }
    if (flags & IFF_PORTSEL) > 0 {
        ret.push(IfaceFlags::Portsel)
    }
    if (flags & IFF_PROMISC) > 0 {
        ret.push(IfaceFlags::Promisc)
    }
    if (flags & IFF_RUNNING) > 0 {
        ret.push(IfaceFlags::Running)
    }
    if (flags & IFF_SLAVE) > 0 {
        ret.push(IfaceFlags::Subordinate)
    }
    if (flags & IFF_UP) > 0 {
        ret.push(IfaceFlags::Up)
    }

    ret
}
