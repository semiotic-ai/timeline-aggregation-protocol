// Copyright 2024 The Graph Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProtocolMode {
    /// Pre-horizon: V1 receipts accepted, V1 aggregation used
    Legacy,
    /// Post-horizon: V2 for new receipts, V1 only for legacy receipt aggregation
    Horizon,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReceiptType {
    /// V1 receipt created before horizon activation (legacy, still needs aggregation)
    LegacyV1,
    /// V2 receipt (collection-based, created after horizon activation)
    V2,
}
