# atlas: fixtures (318 LOC, 7 files) | budget 4096 | rendered 635 tok

## c.c (#1)
struct Point
    int x
    int y
enum Color
typedef struct Point PointAlias
void reset(void)
int add(int a, int b)
static int helper(int x)

## cpp.cpp (#2)
namespace service
class Service
    Service(int count)
    int total() const
    int adjust(int delta)
    int helper(int x)
    int count_
struct Point
    int x
    int y
enum class Level
using Id = int
int add(int a, int b)

## go.go (#3)
const APIVersion = "1.0"
const internalFlag = false
func TopLevel(x int) int
func helper(x int) int
type Service struct
    Name  string
    count int
type Runner interface
    func (s *Service) Run() error
    func (s *Service) reset()

## java.java (#4)
public class Service
    public static final String API_VERSION = "1.0"
    private int count
    public Service(int count)
    public int total()
    private int helper(int x)
interface Runner
    void run()
enum Level

## python.py (#5)
API_VERSION = "1.0"
def top_level(x: int) -> int
async def fetch(url: str) -> Optional[str]
def helper(x: int) -> int
class Service
    def name(self) -> str
    def run(self) -> None
    def helper_method() -> None
class Mark
    name: str
    args: tuple
    description: str | None = None
    _internal: int = 0
def decorating(fn)
def decorated() -> None
class DecoratedService

## rust.rs (#6)
pub const API_VERSION: &str = "1.0"
pub static GLOBAL_FLAG: bool = false
pub fn top_level(x: u32) -> u32
fn helper(x: u32) -> u32
pub struct Service
    names: BTreeMap<String, u32>,
pub enum Level
pub trait Runner
    fn run(&self)
    fn ready(&self) -> bool
    pub fn new() -> Self
    fn run(&self)
pub mod nested
pub type Alias = u64
macro_rules! shout

## typescript.ts (#7)
export const API_VERSION: string = "1.0"
export function topLevel(x: number): number
const arrow = (x: number): number => x * 2
function helper(x: number): number
export interface Options
    budget: number
export type Format = "md" | "json" | "xml"
export enum Level
export class Service
    run(): void
    private helperMethod(): void
export function overloaded(x: number): number
export function overloaded(x: string): string
export function overloaded(x: number | string): number | string
declare function ambient(x: number): void
