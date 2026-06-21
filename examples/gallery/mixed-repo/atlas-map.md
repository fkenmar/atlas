# atlas: mixed-repo (96 LOC, 3 files) | budget 400 | rendered 238 tok

## server.go (#1)
type Request struct
    Features []float64 `json:"features"`
type Response struct
    Score float64 `json:"score"`
type Handler struct
    Weights []float64
    func (h *Handler) Score(features []float64) float64
    func (h *Handler) ServeHTTP(w http.ResponseWriter, r *http.Request)
func main()

## engine.rs (#2)
pub struct Engine
    weights: Vec<f64>,
    pub fn new(weights: Vec<f64>) -> Engine
    pub fn score(&self, features: &[f64]) -> f64
    pub fn rank(&self, rows: &[Vec<f64>]) -> Vec<usize>
pub fn default_engine() -> Engine

## train.py (#3)
def gradient_step(weights: list[float], features: Sequence[float], target: float, lr: float = 0.01) -> list[float]
def train(rows: list[tuple[list[float], float]], epochs: int = 10) -> list[float]
