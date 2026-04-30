from collections import defaultdict
from skidl import Pin, Part, Alias, SchLib, SKIDL, TEMPLATE

from skidl.pin import pin_types

SKIDL_lib_version = '0.0.1'

generate_schematic = SchLib(tool=SKIDL).add_parts(*[
        Part(**{ 'name':'EXT_GPIO', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'EXT_GPIO'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'Connector_JST:JST_XH_B3B-XH-A_1x03_P2.50mm_Vertical', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='1',name='P1',func=pin_types.PASSIVE),
            Pin(num='2',name='P2',func=pin_types.PASSIVE),
            Pin(num='3',name='P3',func=pin_types.PASSIVE)] }),
        Part(**{ 'name':'UEXT', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'UEXT'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'IDC:IDC-Stecker_2x05_P2.54mm_Vertical', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='1',name='P1',func=pin_types.PASSIVE),
            Pin(num='2',name='P2',func=pin_types.PASSIVE),
            Pin(num='3',name='P3',func=pin_types.PASSIVE),
            Pin(num='4',name='P4',func=pin_types.PASSIVE),
            Pin(num='5',name='P5',func=pin_types.PASSIVE),
            Pin(num='6',name='P6',func=pin_types.PASSIVE),
            Pin(num='7',name='P7',func=pin_types.PASSIVE),
            Pin(num='8',name='P8',func=pin_types.PASSIVE),
            Pin(num='9',name='P9',func=pin_types.PASSIVE),
            Pin(num='10',name='P10',func=pin_types.PASSIVE)] }),
        Part(**{ 'name':'SolderBridge', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'SolderBridge'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'Jumper:SolderJumper-2_P1.3mm_Open_RoundedPad1.0x1.5mm', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='1',name='1',func=pin_types.PASSIVE),
            Pin(num='2',name='2',func=pin_types.PASSIVE)] }),
        Part(**{ 'name':'AUX', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'AUX'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'Connector_JST:JST_XH_B4B-XH-A_1x04_P2.50mm_Vertical', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='1',name='P1',func=pin_types.PASSIVE),
            Pin(num='2',name='P2',func=pin_types.PASSIVE),
            Pin(num='3',name='P3',func=pin_types.PASSIVE),
            Pin(num='4',name='P4',func=pin_types.PASSIVE)] }),
        Part(**{ 'name':'OpenTherm', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'OpenTherm'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'TerminalBlock_Phoenix:TerminalBlock_Phoenix_MKDS-1,5-2-5.08_1x02_P5.08mm_Horizontal', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='1',name='P1',func=pin_types.PASSIVE),
            Pin(num='2',name='P2',func=pin_types.PASSIVE)] }),
        Part(**{ 'name':'OLED_SH1106', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'OLED_SH1106'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'SSD1306:128x64OLED-MountingHoles', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='1',name='P1',func=pin_types.PASSIVE),
            Pin(num='2',name='P2',func=pin_types.PASSIVE),
            Pin(num='3',name='P3',func=pin_types.PASSIVE),
            Pin(num='4',name='P4',func=pin_types.PASSIVE)] }),
        Part(**{ 'name':'R', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'R'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'Resistor_SMD:R_0603_1608Metric', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='1',name='1',func=pin_types.PASSIVE),
            Pin(num='2',name='2',func=pin_types.PASSIVE)] }),
        Part(**{ 'name':'LTV-817S', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'LTV-817S'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'Package_SO:SOP-4_3.8x4.1mm_P2.54mm', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='1',name='A',func=pin_types.PASSIVE),
            Pin(num='2',name='K',func=pin_types.PASSIVE),
            Pin(num='3',name='E',func=pin_types.PASSIVE),
            Pin(num='4',name='C',func=pin_types.PASSIVE)] }),
        Part(**{ 'name':'2N7002', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'2N7002'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'Package_TO_SOT_SMD:SOT-23', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='1',name='G',func=pin_types.INPUT),
            Pin(num='2',name='S',func=pin_types.PASSIVE),
            Pin(num='3',name='D',func=pin_types.PASSIVE)] }),
        Part(**{ 'name':'D', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'D'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'Diode_SMD:D_SOD-323', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='1',name='K',func=pin_types.PASSIVE),
            Pin(num='2',name='A',func=pin_types.PASSIVE)] }),
        Part(**{ 'name':'Encoder', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'Encoder'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'Rotary_Encoder:RotaryEncoder_Alps_EC12E-Switch_Vertical_H20mm', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='A',name='A',func=pin_types.PASSIVE),
            Pin(num='C',name='C',func=pin_types.PASSIVE),
            Pin(num='B',name='B',func=pin_types.PASSIVE),
            Pin(num='S1',name='S1',func=pin_types.PASSIVE),
            Pin(num='S2',name='S2',func=pin_types.PASSIVE)] }),
        Part(**{ 'name':'LED', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'LED'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'LED_SMD:LED_0805_2012Metric', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='1',name='A',func=pin_types.PASSIVE),
            Pin(num='2',name='K',func=pin_types.PASSIVE)] }),
        Part(**{ 'name':'MountingHole', 'dest':TEMPLATE, 'tool':SKIDL, 'aliases':Alias({'MountingHole'}), 'ref_prefix':'U', 'fplist':None, 'footprint':'MountingHole:MountingHole_3.2mm_M3_Pad_Via', 'keywords':None, 'description':'', 'datasheet':None, 'pins':[
            Pin(num='1',name='1',func=pin_types.PASSIVE)] })])