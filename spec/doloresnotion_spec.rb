require 'bundler'
Bundler.require(:default)
require_relative('../doloresnotion.rb')
RSpec.describe "doloresnotion" do 
  describe "#convertaltertojson" do
    it "Converts an alter to a json format acceptable by notion" do
    result = convertaltertojson('Catra')
    expect(result).to eq "{\n  \"heading_1\": {\n    \"text\": [\n      {\n        \"type\": \"text\",\n        \"text\": {\n          \"content\": \"Catra\"\n        }\n      }\n    ]\n  }\n}"
end
end

  describe "#getblocksforpage" do 
    it "returns a block's children when it has children" do 
      # ToDo, allow readin
      allow {}
      result = getblocksforpage("REDACTED")
      Net::HTTP.(Net::)
      expect(result).to
    it "returns nil if the block does not have children" do
      double(response)

    end
      
end
end
end