// Package main exposes the scoring service over HTTP.
package main

import (
	"encoding/json"
	"net/http"
)

// Request is a single scoring request.
type Request struct {
	Features []float64 `json:"features"`
}

// Response carries the computed score.
type Response struct {
	Score float64 `json:"score"`
}

// Handler scores incoming feature vectors.
type Handler struct {
	Weights []float64
}

func (h *Handler) Score(features []float64) float64 {
	var total float64
	for i, f := range features {
		if i < len(h.Weights) {
			total += h.Weights[i] * f
		}
	}
	return total
}

func (h *Handler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	var req Request
	_ = json.NewDecoder(r.Body).Decode(&req)
	_ = json.NewEncoder(w).Encode(Response{Score: h.Score(req.Features)})
}

func main() {
	http.ListenAndServe(":8080", &Handler{Weights: []float64{0.5, 0.3, 0.2}})
}
