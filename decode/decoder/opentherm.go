package decoder

import (
	"fmt"
	"math"
	"math/bits"
	"sort"
)

var msgTypeNames = map[int]string{
	0: "Read-Data",
	1: "Write-Data",
	2: "Invalid-Data",
	3: "Reserved",
	4: "Read-Ack",
	5: "Write-Ack",
	6: "Data-Invalid",
	7: "Unknown-DataId",
}

var dataIDNames = map[int]string{
	0:   "Status",
	1:   "Control setpoint",
	2:   "Master configuration",
	3:   "Slave configuration",
	4:   "Remote command",
	5:   "App-specific flags",
	6:   "Remote parameter flags",
	7:   "Cooling control signal",
	8:   "Control setpoint 2",
	9:   "Remote override room setpoint",
	10:  "TSP count",
	11:  "TSP index/value",
	12:  "FHB size",
	13:  "FHB index/value",
	14:  "Max. relative modulation",
	15:  "Max. boiler capacity / min. mod. level",
	16:  "Room setpoint",
	17:  "Relative modulation level",
	18:  "CH water pressure",
	19:  "DHW flow rate",
	20:  "Day of week / time of day",
	21:  "Date",
	22:  "Year",
	23:  "Room setpoint 2",
	24:  "Room temperature",
	25:  "Boiler water temperature",
	26:  "DHW temperature",
	27:  "Outside temperature",
	28:  "Return water temperature",
	29:  "Solar storage temperature",
	30:  "Solar collector temperature",
	31:  "Flow temperature CH2",
	32:  "DHW2 temperature",
	33:  "Exhaust temperature",
	34:  "Boiler heat exchanger temperature",
	35:  "Boiler fan speed setpoint/actual",
	48:  "DHW setpoint boundaries",
	49:  "Max. CH setpoint boundaries",
	56:  "DHW setpoint",
	57:  "Max. CH water setpoint",
	70:  "Status V/H",
	71:  "Control setpoint V/H",
	72:  "Fault flags/code V/H",
	73:  "Diagnostic code V/H",
	74:  "Config/member-ID V/H",
	75:  "OpenTherm version V/H",
	76:  "Product version V/H",
	77:  "Relative ventilation position",
	78:  "Relative humidity exhaust",
	79:  "CO2 level exhaust",
	80:  "Supply inlet temperature",
	81:  "Supply outlet temperature",
	82:  "Exhaust inlet temperature",
	83:  "Exhaust outlet temperature",
	84:  "Exhaust fan speed RPM",
	85:  "Supply fan speed RPM",
	86:  "Remote parameter V/H",
	87:  "Nominal ventilation value",
	88:  "TSP count V/H",
	89:  "TSP index/value V/H",
	90:  "FHB size V/H",
	91:  "FHB index/value V/H",
	100: "Remote override function",
	115: "OEM diagnostic code",
	116: "Burner starts",
	117: "CH pump starts",
	118: "DHW pump/valve starts",
	119: "DHW burner starts",
	120: "Burner operation hours",
	121: "CH pump operation hours",
	122: "DHW pump/valve operation hours",
	123: "DHW burner operation hours",
	124: "OpenTherm version master",
	125: "OpenTherm version slave",
	126: "Master product version",
	127: "Slave product version",
}

func Decode(samples []Sample) {
	sampleInterval := samples[1].Time - samples[0].Time
	fmt.Printf("Loaded %d samples, interval=%.3fus, duration=%.3fs\n",
		len(samples), sampleInterval*1e6, samples[len(samples)-1].Time-samples[0].Time)

	vmin, vmax := samples[0].Voltage, samples[0].Voltage
	for _, s := range samples {
		if s.Voltage < vmin {
			vmin = s.Voltage
		}
		if s.Voltage > vmax {
			vmax = s.Voltage
		}
	}
	fmt.Printf("Voltage range: %.3f - %.3f V\n", vmin, vmax)

	packets := findPackets(samples, 6.1, 0.005)
	fmt.Printf("Found %d packets\n\n", len(packets))

	analyzeBitTiming(packets)
	fmt.Println()

	fmt.Println("=== OpenTherm Protocol Decode ===")
	fmt.Println("Frame: 1 start bit + 32 data bits + 1 stop bit = 34 bits")
	fmt.Println("Data: Parity(1) + MsgType(3) + Spare(4) + DataID(8) + Value(16)")
	fmt.Println()

	for i, pkt := range packets {
		kind := "RSP"
		if pkt.IsRequest {
			kind = "REQ"
		}
		rawBits := decodeManchester(pkt)

		if len(rawBits) < 34 {
			fmt.Printf("Packet %2d [%s] t=%.6fs: only %d bits (need 34)\n", i, kind, pkt.StartTime, len(rawBits))
			continue
		}

		startBit := rawBits[0]
		stopBit := rawBits[33]
		dataBits := rawBits[1:33]

		var frame uint32
		for _, b := range dataBits {
			frame = (frame << 1) | uint32(b)
		}

		parity := (frame >> 31) & 1
		msgType := int((frame >> 28) & 0x7)
		spare := (frame >> 24) & 0xF
		dataID := int((frame >> 16) & 0xFF)
		dataValue := frame & 0xFFFF
		dataHi := int((dataValue >> 8) & 0xFF)
		dataLo := int(dataValue & 0xFF)

		ones := bits.OnesCount32(frame)
		parityOK := ones%2 == 0

		f88 := float64(int16(dataValue)) / 256.0

		msgTypeName := msgTypeNames[msgType]
		idName := dataIDNames[dataID]
		if idName == "" {
			idName = "Unknown"
		}

		fmt.Printf("Pkt %2d [%s] t=%8.4fs peak=%5.1fV | ", i, kind, pkt.StartTime, pkt.PeakV)
		fmt.Printf("Start=%d Stop=%d ", startBit, stopBit)
		fmt.Printf("Parity=%d(%s) ", parity, map[bool]string{true: "OK", false: "ERR"}[parityOK])
		fmt.Printf("Spare=%X\n", spare)
		fmt.Printf("       %-15s ID=%3d (0x%02X) %-38s Value=0x%04X (%d) HI=%d LO=%d f8.8=%.2f\n",
			msgTypeName, dataID, dataID, idName, dataValue, dataValue, dataHi, dataLo, f88)
		fmt.Println()
	}
}

func findPackets(samples []Sample, activityThreshold, gapThreshold float64) []Packet {
	var packets []Packet
	inPacket := false
	var startIdx int
	var pkt Packet

	for i, s := range samples {
		if s.Voltage > activityThreshold {
			if !inPacket {
				pkt = Packet{StartTime: s.Time, PeakV: s.Voltage}
				startIdx = i
				inPacket = true
			}
			pkt.EndTime = s.Time
			if s.Voltage > pkt.PeakV {
				pkt.PeakV = s.Voltage
			}
		} else if inPacket && (s.Time-pkt.EndTime) > gapThreshold {
			pkt.IsRequest = pkt.PeakV > 8.0
			margin := int(0.001 / (samples[1].Time - samples[0].Time))
			lo := startIdx - margin
			if lo < 0 {
				lo = 0
			}
			hi := i + margin
			if hi > len(samples) {
				hi = len(samples)
			}
			pkt.Samples = samples[lo:hi]
			packets = append(packets, pkt)
			inPacket = false
		}
	}
	if inPacket {
		pkt.IsRequest = pkt.PeakV > 8.0
		margin := int(0.001 / (samples[1].Time - samples[0].Time))
		lo := startIdx - margin
		if lo < 0 {
			lo = 0
		}
		pkt.Samples = samples[lo:]
		packets = append(packets, pkt)
	}

	return packets
}

func findEdges(samples []Sample, threshold float64, hysteresis float64) []Edge {
	var edges []Edge
	highThresh := threshold + hysteresis
	lowThresh := threshold - hysteresis
	isHigh := false

	for _, s := range samples {
		if !isHigh && s.Voltage > highThresh {
			isHigh = true
			edges = append(edges, Edge{Time: s.Time, Rising: true})
		} else if isHigh && s.Voltage < lowThresh {
			isHigh = false
			edges = append(edges, Edge{Time: s.Time, Rising: false})
		}
	}
	return edges
}

func analyzeBitTiming(packets []Packet) {
	var allPulseWidths []float64

	for _, pkt := range packets {
		threshold := 6.5
		hysteresis := 0.3
		if pkt.IsRequest {
			threshold = 8.0
			hysteresis = 0.5
		}

		edges := findEdges(pkt.Samples, threshold, hysteresis)
		for i := 1; i < len(edges); i++ {
			dt := edges[i].Time - edges[i-1].Time
			allPulseWidths = append(allPulseWidths, dt)
		}
	}

	if len(allPulseWidths) == 0 {
		fmt.Println("No pulses found!")
		return
	}

	sort.Float64s(allPulseWidths)

	minW := allPulseWidths[0]
	maxW := allPulseWidths[len(allPulseWidths)-1]
	fmt.Printf("Pulse width range: %.1f - %.1fus\n", minW*1e6, maxW*1e6)

	boundary := minW * 1.5
	var singles, doubles []float64
	for _, w := range allPulseWidths {
		if w < boundary {
			singles = append(singles, w)
		} else {
			doubles = append(doubles, w)
		}
	}

	singleAvg := avg(singles)
	doubleAvg := avg(doubles)

	fmt.Printf("Single pulse: count=%d avg=%.1fus\n", len(singles), singleAvg*1e6)
	fmt.Printf("Double pulse: count=%d avg=%.1fus\n", len(doubles), doubleAvg*1e6)
	fmt.Printf("Ratio double/single: %.2f\n", doubleAvg/singleAvg)

	halfBit := singleAvg
	bitPeriod := halfBit * 2
	baudRate := 1.0 / bitPeriod

	fmt.Printf("\nManchester encoding detected:\n")
	fmt.Printf("  Half-bit period: %.1fus\n", halfBit*1e6)
	fmt.Printf("  Bit period:      %.1fus\n", bitPeriod*1e6)
	fmt.Printf("  Baud rate:       %.0f baud\n", baudRate)

	standardRates := []float64{300, 600, 1200, 2400, 4800, 9600, 1000, 1042}
	closest := standardRates[0]
	for _, r := range standardRates {
		if math.Abs(r-baudRate) < math.Abs(closest-baudRate) {
			closest = r
		}
	}
	fmt.Printf("  Closest standard: %.0f baud (%.1f%% off)\n",
		closest, math.Abs(closest-baudRate)/closest*100)
}

func decodeManchester(pkt Packet) []int {
	threshold := 6.5
	hysteresis := 0.3
	if pkt.IsRequest {
		threshold = 8.0
		hysteresis = 0.5
	}

	edges := findEdges(pkt.Samples, threshold, hysteresis)
	if len(edges) < 2 {
		return nil
	}

	var pulseWidths []float64
	for i := 1; i < len(edges); i++ {
		pulseWidths = append(pulseWidths, edges[i].Time-edges[i-1].Time)
	}
	sort.Float64s(pulseWidths)
	boundary := pulseWidths[0] * 1.5
	var singles []float64
	for _, w := range pulseWidths {
		if w < boundary {
			singles = append(singles, w)
		}
	}
	halfBit := avg(singles)

	var decodedBits []int
	atMidBit := false

	for i := 1; i < len(edges); i++ {
		dt := edges[i].Time - edges[i-1].Time

		if dt < halfBit*1.5 {
			atMidBit = !atMidBit
			if atMidBit {
				if edges[i].Rising {
					decodedBits = append(decodedBits, 0)
				} else {
					decodedBits = append(decodedBits, 1)
				}
			}
		} else {
			atMidBit = true
			if edges[i].Rising {
				decodedBits = append(decodedBits, 0)
			} else {
				decodedBits = append(decodedBits, 1)
			}
		}
	}

	return decodedBits
}

func avg(values []float64) float64 {
	if len(values) == 0 {
		return 0
	}
	sum := 0.0
	for _, v := range values {
		sum += v
	}
	return sum / float64(len(values))
}
