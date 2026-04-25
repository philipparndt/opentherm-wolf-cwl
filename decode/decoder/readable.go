package decoder

import (
	"fmt"
	"math/bits"
	"strings"
)

// Value types for OpenTherm data IDs
type ValueType int

const (
	TypeFlag8Flag8 ValueType = iota
	TypeF88
	TypeU8HI
	TypeU8LO
	TypeU16
	TypeProductVersion
	TypeConfigMemberID
	TypeTSP
)

// DataIDInfo describes how to interpret a specific OpenTherm data ID
type DataIDInfo struct {
	DisplayName string
	SnakeID     string
	Type        ValueType
	Unit        string
}

var dataIDInfo = map[int]DataIDInfo{
	0:   {"Status", "status", TypeFlag8Flag8, ""},
	2:   {"Master configuration", "master_configuration", TypeConfigMemberID, ""},
	3:   {"Slave configuration", "slave_configuration", TypeConfigMemberID, ""},
	11:  {"TSP index/value", "tsp_index_value", TypeTSP, ""},
	17:  {"Relative modulation level", "relative_modulation_level", TypeF88, "%"},
	18:  {"CH water pressure", "ch_water_pressure", TypeF88, "bar"},
	24:  {"Room temperature", "room_temperature", TypeF88, "°C"},
	25:  {"Boiler water temperature", "boiler_water_temperature", TypeF88, "°C"},
	26:  {"DHW temperature", "dhw_temperature", TypeF88, "°C"},
	27:  {"Outside temperature", "outside_temperature", TypeF88, "°C"},
	28:  {"Return water temperature", "return_water_temperature", TypeF88, "°C"},
	70:  {"Status V/H", "status_vh", TypeFlag8Flag8, ""},
	71:  {"Control setpoint V/H", "control_setpoint_vh", TypeU8LO, ""},
	72:  {"Fault flags/code V/H", "fault_flags_code_vh", TypeFlag8Flag8, ""},
	73:  {"Diagnostic code V/H", "diagnostic_code_vh", TypeU16, ""},
	74:  {"Config/member-ID V/H", "config_member_id_vh", TypeConfigMemberID, ""},
	75:  {"OpenTherm version V/H", "opentherm_version_vh", TypeF88, ""},
	76:  {"Product version V/H", "product_version_vh", TypeProductVersion, ""},
	77:  {"Relative ventilation", "relative_ventilation", TypeU8LO, "%"},
	78:  {"Relative humidity exhaust", "relative_humidity_exhaust", TypeU8LO, "%"},
	79:  {"CO2 level exhaust", "co2_level_exhaust", TypeU16, "ppm"},
	80:  {"Supply inlet temperature", "supply_inlet_temperature", TypeF88, "°C"},
	81:  {"Supply outlet temperature", "supply_outlet_temperature", TypeF88, "°C"},
	82:  {"Exhaust inlet temperature", "exhaust_inlet_temperature", TypeF88, "°C"},
	83:  {"Exhaust outlet temperature", "exhaust_outlet_temperature", TypeF88, "°C"},
	84:  {"Exhaust fan speed RPM", "exhaust_fan_speed_rpm", TypeU16, "rpm"},
	85:  {"Supply fan speed RPM", "supply_fan_speed_rpm", TypeU16, "rpm"},
	87:  {"Nominal ventilation value", "nominal_ventilation_value", TypeU8LO, ""},
	88:  {"TSP count V/H", "tsp_count_vh", TypeU8LO, ""},
	89:  {"TSP index/value V/H", "tsp_index_value_vh", TypeTSP, ""},
	116: {"Burner starts", "burner_starts", TypeU16, ""},
	117: {"CH pump starts", "ch_pump_starts", TypeU16, ""},
	118: {"DHW pump/valve starts", "dhw_pump_valve_starts", TypeU16, ""},
	119: {"DHW burner starts", "dhw_burner_starts", TypeU16, ""},
	120: {"Burner operation hours", "burner_operation_hours", TypeU16, "h"},
	121: {"CH pump operation hours", "ch_pump_operation_hours", TypeU16, "h"},
	122: {"DHW pump/valve operation hours", "dhw_pump_valve_operation_hours", TypeU16, "h"},
	123: {"DHW burner operation hours", "dhw_burner_operation_hours", TypeU16, "h"},
	124: {"OpenTherm version master", "opentherm_version_master", TypeF88, ""},
	125: {"OpenTherm version slave", "opentherm_version_slave", TypeF88, ""},
	126: {"Master product version", "master_product_version", TypeProductVersion, ""},
	127: {"Slave product version", "slave_product_version", TypeProductVersion, ""},
}

func formatValue(dataID int, value uint16) string {
	info, ok := dataIDInfo[dataID]
	if !ok {
		return fmt.Sprintf("value=0x%04X", value)
	}

	hi := uint8(value >> 8)
	lo := uint8(value & 0xFF)

	switch info.Type {
	case TypeFlag8Flag8:
		return formatFlags(dataID, hi, lo)
	case TypeF88:
		return formatF88(value, info.Unit)
	case TypeU8HI:
		return formatU8(hi, info.Unit)
	case TypeU8LO:
		return formatU8(lo, info.Unit)
	case TypeU16:
		return formatU16(value, info.Unit)
	case TypeProductVersion:
		return formatProductVersion(hi, lo)
	case TypeConfigMemberID:
		return formatConfigMemberID(hi, lo)
	case TypeTSP:
		return formatTSP(hi, lo)
	default:
		return fmt.Sprintf("value=0x%04X", value)
	}
}

func formatF88(value uint16, unit string) string {
	f := float64(int16(value)) / 256.0
	if unit != "" {
		return fmt.Sprintf("%.1f %s", f, unit)
	}
	return fmt.Sprintf("%.1f", f)
}

func formatU8(value uint8, unit string) string {
	if unit != "" {
		return fmt.Sprintf("%d %s", value, unit)
	}
	return fmt.Sprintf("%d", value)
}

func formatU16(value uint16, unit string) string {
	if unit != "" {
		return fmt.Sprintf("%d %s", value, unit)
	}
	return fmt.Sprintf("%d", value)
}

func formatProductVersion(typ, version uint8) string {
	return fmt.Sprintf("type=%d version=%d", typ, version)
}

func formatConfigMemberID(flags, memberID uint8) string {
	return fmt.Sprintf("flags=0x%02X member-id=%d", flags, memberID)
}

func formatTSP(index, value uint8) string {
	return fmt.Sprintf("[%d] = %d", index, value)
}

var statusVHBitsHI = []string{
	"fault",
	"ventilation",
	"cooling",
	"dhw",
	"filter",
	"diag",
}

// Master control flags for Status V/H (HI byte of request)
var statusVHMasterBits = []string{
	"ch",
	"bypass",
	"cooling",
	"dhw",
	"filter",
	"diag",
}

var statusBitsHI = []string{
	"fault",
	"ch",
	"dhw",
	"flame",
	"cooling",
	"ch2",
	"diag",
}

var statusBitsLO = []string{
	"ch",
	"dhw",
	"cooling",
	"otc",
	"ch2",
}

func formatFlags(dataID int, hi, lo uint8) string {
	switch dataID {
	case 70:
		return formatFlagBits(hi, statusVHBitsHI)
	case 72:
		return formatFaultVH(hi, lo)
	case 0:
		return formatFlagBits(hi, statusBitsHI)
	default:
		return fmt.Sprintf("HI=0x%02X LO=0x%02X", hi, lo)
	}
}

func formatFaultVH(flags, code uint8) string {
	return fmt.Sprintf("flags=0x%02X code=%d", flags, code)
}

func formatFlagBits(value uint8, names []string) string {
	var parts []string
	for i, name := range names {
		if name == "" {
			continue
		}
		state := "off"
		if value&(1<<uint(i)) != 0 {
			state = "on"
		}
		parts = append(parts, fmt.Sprintf("%s=%s", name, state))
	}
	return strings.Join(parts, " ")
}

// Exchange represents a paired request/response or a single unpaired packet
type Exchange struct {
	DataID     int
	MsgType    int
	ReqMsgType int    // message type of the request (0=Read-Data, 1=Write-Data)
	ReqValue   uint16 // value sent in the request
	Value      uint16 // value from the response (or request if unpaired)
	Paired     bool
	IsReqOnly  bool
}

// CaptureResult holds extracted data from a capture file
type CaptureResult struct {
	Exchanges []Exchange
	Duration  float64
}

// ExtractExchanges decodes samples and returns paired exchanges
func ExtractExchanges(samples []Sample) CaptureResult {
	packets := findPackets(samples, 6.1, 0.005)

	type decodedPacket struct {
		isRequest bool
		msgType   int
		dataID    int
		value     uint16
	}

	var decoded []decodedPacket

	for _, pkt := range packets {
		rawBits := decodeManchester(pkt)
		if len(rawBits) < 34 {
			continue
		}

		dataBits := rawBits[1:33]
		var frame uint32
		for _, b := range dataBits {
			frame = (frame << 1) | uint32(b)
		}

		ones := bits.OnesCount32(frame)
		if ones%2 != 0 {
			continue
		}

		msgType := int((frame >> 28) & 0x7)
		dataID := int((frame >> 16) & 0xFF)
		dataValue := uint16(frame & 0xFFFF)

		decoded = append(decoded, decodedPacket{
			isRequest: pkt.IsRequest,
			msgType:   msgType,
			dataID:    dataID,
			value:     dataValue,
		})
	}

	// Pair consecutive REQ+RSP by data ID
	var exchanges []Exchange
	used := make([]bool, len(decoded))

	for i := 0; i < len(decoded); i++ {
		if used[i] {
			continue
		}
		req := decoded[i]
		if req.isRequest && i+1 < len(decoded) && !used[i+1] {
			rsp := decoded[i+1]
			if !rsp.isRequest && rsp.dataID == req.dataID {
				exchanges = append(exchanges, Exchange{
					DataID:     rsp.dataID,
					MsgType:    rsp.msgType,
					ReqMsgType: req.msgType,
					ReqValue:   req.value,
					Value:      rsp.value,
					Paired:     true,
				})
				used[i] = true
				used[i+1] = true
				continue
			}
		}
		used[i] = true
		exchanges = append(exchanges, Exchange{
			DataID:     req.dataID,
			MsgType:    req.msgType,
			ReqMsgType: req.msgType,
			Value:      req.value,
			Paired:     false,
			IsReqOnly:  req.isRequest,
		})
	}

	duration := samples[len(samples)-1].Time - samples[0].Time
	return CaptureResult{Exchanges: exchanges, Duration: duration}
}

// FormatExchange returns the formatted display string for an exchange
func FormatExchange(ex Exchange) string {
	return formatValue(ex.DataID, ex.Value)
}

// statusExchange holds formatted master and slave parts separately
type statusExchange struct {
	master string
	slave  string
}

// formatStatusExchange returns master/slave parts for status IDs, or nil for non-status
func formatStatusExchange(ex Exchange) *statusExchange {
	if !ex.Paired {
		return nil
	}
	if ex.DataID == 70 {
		reqHi := uint8(ex.ReqValue >> 8)
		return &statusExchange{
			master: formatFlagBits(reqHi, statusVHMasterBits),
			slave:  formatValue(ex.DataID, ex.Value),
		}
	}
	if ex.DataID == 0 {
		reqLo := uint8(ex.ReqValue & 0xFF)
		return &statusExchange{
			master: formatFlagBits(reqLo, statusBitsLO),
			slave:  formatValue(ex.DataID, ex.Value),
		}
	}
	return nil
}

// FormatExchangeFull returns formatted string including request value for status IDs
func FormatExchangeFull(ex Exchange) string {
	if se := formatStatusExchange(ex); se != nil {
		return fmt.Sprintf("master: %s | slave: %s", se.master, se.slave)
	}
	return formatValue(ex.DataID, ex.Value)
}

// DisplayName returns the human-readable name for a data ID
func DisplayName(dataID int) string {
	return displayName(dataID)
}

// deduplicatedEntry tracks a value across polling cycles
type deduplicatedEntry struct {
	key           string
	name          string
	values        []string  // all distinct formatted values in order
	rawValues     []uint16  // all distinct raw values in order
	isReqOnly     bool
	hasWrites     bool      // true if any Write-Data was seen for this key
	isStatus      bool      // true if this is a status exchange with master/slave parts
	masterValues  []string  // distinct master flag values (for status IDs)
	slaveValues   []string  // distinct slave flag values (for status IDs)
}

// ExchangeKey returns a unique key for an exchange (includes TSP index for TSP registers)
func ExchangeKey(ex Exchange) string {
	prefix := ""
	if ex.ReqMsgType == 1 {
		prefix = "w:"
	}
	if ex.DataID == 89 || ex.DataID == 11 {
		index := uint8(ex.Value >> 8)
		return fmt.Sprintf("%s%d:tsp%d", prefix, ex.DataID, index)
	}
	return fmt.Sprintf("%s%d", prefix, ex.DataID)
}

// IsWrite returns true if the exchange was initiated by a Write-Data request
func IsWrite(ex Exchange) bool {
	return ex.ReqMsgType == 1
}

// DecodeReadable prints deduplicated human-readable output from samples
func DecodeReadable(samples []Sample) {
	result := ExtractExchanges(samples)

	// Deduplicate: track first and last value per key
	entries := make(map[string]*deduplicatedEntry)
	var order []string

	for _, ex := range result.Exchanges {
		key := ExchangeKey(ex)
		val := FormatExchangeFull(ex)
		se := formatStatusExchange(ex)

		if e, exists := entries[key]; exists {
			if e.isStatus && se != nil {
				if se.master != e.masterValues[len(e.masterValues)-1] {
					e.masterValues = append(e.masterValues, se.master)
				}
				if se.slave != e.slaveValues[len(e.slaveValues)-1] {
					e.slaveValues = append(e.slaveValues, se.slave)
				}
			} else if val != e.values[len(e.values)-1] {
				e.values = append(e.values, val)
				e.rawValues = append(e.rawValues, ex.Value)
			}
		} else {
			entry := &deduplicatedEntry{
				key:       key,
				name:      displayName(ex.DataID),
				values:    []string{val},
				rawValues: []uint16{ex.Value},
				isReqOnly: !ex.Paired && ex.IsReqOnly,
				hasWrites: IsWrite(ex),
			}
			if se != nil {
				entry.isStatus = true
				entry.masterValues = []string{se.master}
				entry.slaveValues = []string{se.slave}
			}
			order = append(order, key)
			entries[key] = entry
		}
	}

	// Count cycles (approximate: count how many times the first key appears)
	cycles := 0
	if len(order) > 0 {
		firstKey := order[0]
		for _, ex := range result.Exchanges {
			if ExchangeKey(ex) == firstKey {
				cycles++
			}
		}
	}

	// Summary
	if cycles > 1 {
		fmt.Printf("Decoded %d exchanges, %d cycles (%.1fs capture)\n\n",
			len(result.Exchanges), cycles, result.Duration)
	} else {
		fmt.Printf("Decoded %d exchanges (%.1fs capture)\n\n",
			len(result.Exchanges), result.Duration)
	}

	// Find max name length
	maxNameLen := 0
	for _, key := range order {
		e := entries[key]
		if len(e.name) > maxNameLen {
			maxNameLen = len(e.name)
		}
	}

	// Print
	indent := strings.Repeat(" ", maxNameLen+3)
	for _, key := range order {
		e := entries[key]
		label := e.name + ":"

		if e.isStatus {
			masterChanged := len(e.masterValues) > 1
			slaveChanged := len(e.slaveValues) > 1

			arrowIndent := indent + "     -> "

			masterSuffix := ""
			if masterChanged {
				masterSuffix = "  (changed)"
			}
			slaveSuffix := ""
			if slaveChanged {
				slaveSuffix = "  (changed)"
			}

			fmt.Printf("%-*s  master: %s%s\n", maxNameLen+1, label,
				strings.Join(e.masterValues, "\n"+arrowIndent), masterSuffix)
			fmt.Printf("%sslave:  %s%s\n", indent,
				strings.Join(e.slaveValues, "\n"+arrowIndent), slaveSuffix)
		} else {
			tag := ""
			if e.hasWrites {
				tag = "[WRITE] "
			}

			if len(e.values) > 1 {
				fmt.Printf("%-*s  %s%s  (changed)\n", maxNameLen+1, label, tag, strings.Join(e.values, " -> "))
			} else if e.isReqOnly {
				fmt.Printf("%-*s  [REQ] %s\n", maxNameLen+1, label, e.values[0])
			} else {
				fmt.Printf("%-*s  %s%s\n", maxNameLen+1, label, tag, e.values[0])
			}
		}
	}
}

func displayName(dataID int) string {
	if info, ok := dataIDInfo[dataID]; ok {
		return info.DisplayName
	}
	if name, ok := dataIDNames[dataID]; ok {
		return name
	}
	return fmt.Sprintf("Unknown (ID %d)", dataID)
}
