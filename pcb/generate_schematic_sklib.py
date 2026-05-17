from collections import defaultdict
from skidl import Pin, Part, Alias, SchLib, SKIDL, TEMPLATE

from skidl.pin import pin_types

SKIDL_lib_version = '0.0.1'

generate_schematic = SchLib(tool=SKIDL).add_parts(*[
        Part(**{ 'name':'Conn_01x03_Pin', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'Conn_01x03_Pin'}), 'ref_prefix':'J', 'fplist':[''], 'footprint':'Connector_JST:JST_XH_B3B-XH-A_1x03_P2.50mm_Vertical', 'keywords':'connector', 'description':'Generic connector, single row, 01x03, script generated', 'datasheet':'', 'pins':[
            Pin(num='1',name='Pin_1',func=pin_types.PASSIVE,unit=1),
            Pin(num='2',name='Pin_2',func=pin_types.PASSIVE,unit=1),
            Pin(num='3',name='Pin_3',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'Conn_02x05_Odd_Even', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'Conn_02x05_Odd_Even'}), 'ref_prefix':'J', 'fplist':[''], 'footprint':'IDC:IDC-Stecker_2x05_P2.54mm_Vertical', 'keywords':'connector', 'description':'Generic connector, double row, 02x05, odd/even pin numbering scheme (row 1 odd numbers, row 2 even numbers), script generated (kicad-library-utils/schlib/autogen/connector/)', 'datasheet':'', 'pins':[
            Pin(num='1',name='Pin_1',func=pin_types.PASSIVE,unit=1),
            Pin(num='2',name='Pin_2',func=pin_types.PASSIVE,unit=1),
            Pin(num='3',name='Pin_3',func=pin_types.PASSIVE,unit=1),
            Pin(num='4',name='Pin_4',func=pin_types.PASSIVE,unit=1),
            Pin(num='5',name='Pin_5',func=pin_types.PASSIVE,unit=1),
            Pin(num='6',name='Pin_6',func=pin_types.PASSIVE,unit=1),
            Pin(num='7',name='Pin_7',func=pin_types.PASSIVE,unit=1),
            Pin(num='8',name='Pin_8',func=pin_types.PASSIVE,unit=1),
            Pin(num='9',name='Pin_9',func=pin_types.PASSIVE,unit=1),
            Pin(num='10',name='Pin_10',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'SolderJumper_2_Open', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'SolderJumper_2_Open'}), 'ref_prefix':'JP', 'fplist':[''], 'footprint':'Jumper:SolderJumper-2_P1.3mm_Open_RoundedPad1.0x1.5mm', 'keywords':'solder jumper SPST', 'description':'Solder Jumper, 2-pole, open', 'datasheet':'', 'pins':[
            Pin(num='1',name='A',func=pin_types.PASSIVE,unit=1),
            Pin(num='2',name='B',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'SolderJumper_2_Bridged', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'SolderJumper_2_Bridged'}), 'ref_prefix':'JP', 'fplist':[''], 'footprint':'Jumper:SolderJumper-2_P1.3mm_Bridged_RoundedPad1.0x1.5mm', 'keywords':'solder jumper SPST', 'description':'Solder Jumper, 2-pole, closed/bridged', 'datasheet':'', 'pins':[
            Pin(num='1',name='A',func=pin_types.PASSIVE,unit=1),
            Pin(num='2',name='B',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'Conn_01x04_Pin', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'Conn_01x04_Pin'}), 'ref_prefix':'J', 'fplist':[''], 'footprint':'Connector_JST:JST_XH_B4B-XH-A_1x04_P2.50mm_Vertical', 'keywords':'connector', 'description':'Generic connector, single row, 01x04, script generated', 'datasheet':'', 'pins':[
            Pin(num='1',name='Pin_1',func=pin_types.PASSIVE,unit=1),
            Pin(num='2',name='Pin_2',func=pin_types.PASSIVE,unit=1),
            Pin(num='3',name='Pin_3',func=pin_types.PASSIVE,unit=1),
            Pin(num='4',name='Pin_4',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'Conn_01x02_Pin', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'Conn_01x02_Pin'}), 'ref_prefix':'J', 'fplist':[''], 'footprint':'TerminalBlock_Phoenix:TerminalBlock_Phoenix_MKDS-1,5-2-5.08_1x02_P5.08mm_Horizontal', 'keywords':'connector', 'description':'Generic connector, single row, 01x02, script generated', 'datasheet':'', 'pins':[
            Pin(num='1',name='Pin_1',func=pin_types.PASSIVE,unit=1),
            Pin(num='2',name='Pin_2',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'D', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'D'}), 'ref_prefix':'D', 'fplist':[''], 'footprint':'Diode_SMD:D_SOD-323F', 'keywords':'diode', 'description':'Diode', 'datasheet':'', 'pins':[
            Pin(num='1',name='K',func=pin_types.PASSIVE,unit=1),
            Pin(num='2',name='A',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'D_Zener', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'D_Zener'}), 'ref_prefix':'D', 'fplist':[''], 'footprint':'Diode_SMD:D_SOD-123', 'keywords':'diode', 'description':'Zener diode', 'datasheet':'', 'pins':[
            Pin(num='1',name='K',func=pin_types.PASSIVE,unit=1),
            Pin(num='2',name='A',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'BC858', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'BC858'}), 'ref_prefix':'Q', 'fplist':['', 'Package_TO_SOT_SMD:SOT-23'], 'footprint':'Package_TO_SOT_SMD:SOT-23', 'keywords':'PNP transistor', 'description':'0.1A Ic, 30V Vce, PNP Transistor, SOT-23', 'datasheet':'https://www.onsemi.com/pub/Collateral/BC860-D.pdf', 'pins':[
            Pin(num='1',name='B',func=pin_types.INPUT,unit=1),
            Pin(num='2',name='E',func=pin_types.PASSIVE,unit=1),
            Pin(num='3',name='C',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'R', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'R'}), 'ref_prefix':'R', 'fplist':[''], 'footprint':'Resistor_SMD:R_0603_1608Metric', 'keywords':'R res resistor', 'description':'Resistor', 'datasheet':'', 'pins':[
            Pin(num='1',func=pin_types.PASSIVE,unit=1),
            Pin(num='2',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'PC817', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'PC817'}), 'ref_prefix':'U', 'fplist':['Package_DIP:DIP-4_W7.62mm'], 'footprint':'Package_SO:SOP-4_7.5x4.1mm_P2.54mm', 'keywords':'NPN DC Optocoupler', 'description':'DC Optocoupler, Vce 35V, CTR 50-300%, DIP-4', 'datasheet':'http://www.soselectronic.cz/a_info/resource/d/pc817.pdf', 'pins':[
            Pin(num='1',func=pin_types.PASSIVE,unit=1),
            Pin(num='2',func=pin_types.PASSIVE,unit=1),
            Pin(num='3',func=pin_types.PASSIVE,unit=1),
            Pin(num='4',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'RotaryEncoder_Switch', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'RotaryEncoder_Switch'}), 'ref_prefix':'SW', 'fplist':[''], 'footprint':'Rotary_Encoder:RotaryEncoder_Alps_EC12E-Switch_Vertical_H20mm', 'keywords':'rotary switch encoder switch push button', 'description':'Rotary encoder, dual channel, incremental quadrate outputs, with switch', 'datasheet':'', 'pins':[
            Pin(num='A',name='A',func=pin_types.PASSIVE,unit=1),
            Pin(num='B',name='B',func=pin_types.PASSIVE,unit=1),
            Pin(num='C',name='C',func=pin_types.PASSIVE,unit=1),
            Pin(num='S1',name='S1',func=pin_types.PASSIVE,unit=1),
            Pin(num='S2',name='S2',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'LED', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'LED'}), 'ref_prefix':'D', 'fplist':[''], 'footprint':'LED_SMD:LED_0805_2012Metric', 'keywords':'LED diode', 'description':'Light emitting diode', 'datasheet':'', 'pins':[
            Pin(num='1',name='K',func=pin_types.PASSIVE,unit=1),
            Pin(num='2',name='A',func=pin_types.PASSIVE,unit=1)], 'unit_defs':[] }),
        Part(**{ 'name':'MountingHole_Pad', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'MountingHole_Pad'}), 'ref_prefix':'H', 'fplist':[''], 'footprint':'MountingHole:MountingHole_3.2mm_M3_Pad_Via', 'keywords':'mounting hole', 'description':'Mounting Hole with connection', 'datasheet':'', 'pins':[
            Pin(num='1',name='1',func=pin_types.INPUT,unit=1)], 'unit_defs':[] })])