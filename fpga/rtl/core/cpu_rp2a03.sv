module cpu_rp2a03 {
  input  logic        clk,
  input  logic        rst_n,

  output logic [15:0] addr,
  input  logic [7:0]  data_in,
  output logic [7:0]  data_out,
  output logic [7:0]  we
};

  logic [7:0]  A;
  logic [7:0]  X;
  logic [7:0]  Y;
  logic [7:0]  SP;
  logic [7:0]  P;
  logic [15:0] PC;

endmodule
