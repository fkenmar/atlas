// Fixture for queries/go/tags.scm — one construct per extraction rule.
// Not compiled by cargo; it only needs to parse.

package service

import (
	"fmt"
	"strings"
)

const APIVersion = "1.0"

const internalFlag = false

// TopLevel is an exported free function.
func TopLevel(x int) int {
	return helper(x)
}

func helper(x int) int {
	return x + 1
}

// Service is an exported struct type.
type Service struct {
	Name  string
	count int
}

// Runner is an exported interface type.
type Runner interface {
	Run() error
}

// Run is a method with a pointer receiver.
func (s *Service) Run() error {
	fmt.Println(s.Name)
	return nil
}

func (s *Service) reset() {
	s.count = strings.Count(s.Name, "x")
}
