package decoder

type Sample struct {
	Time    float64
	Voltage float64
}

type Edge struct {
	Time   float64
	Rising bool
}

type Packet struct {
	StartTime float64
	EndTime   float64
	PeakV     float64
	IsRequest bool
	Samples   []Sample
}
